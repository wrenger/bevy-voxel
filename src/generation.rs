use std::cell::RefCell;
use std::f32::consts::PI;
use std::ops::Range;

use bevy::prelude::*;
use noise::{MultiFractal, NoiseFn, RidgedMulti, Simplex};

use crate::block::BlockId;
use crate::chunk::Chunk;
use crate::util::{for_uvec3, RangeExt};

const MIN_HEIGHT: isize = -128;
const MAX_HEIGHT: isize = 128;
const DIRT_HEIGHT: usize = 2;

#[derive(Debug, Clone)]
pub struct NoiseParam {
    pub octaves: usize,
    pub frequency: f32,
    pub lacunarity: f32,
    pub persistence: f32,
    pub attenuation: f32,
}

/// World generation parameters
#[derive(Debug, Resource, Clone)]
pub struct WorldGen {
    /// Base 3d noise
    pub base: NoiseParam,
    pub base_limit: Range<f32>,
    pub base_strength: f32,
    /// Cave 3d noise
    pub cave_limit: Range<f32>,

    /// The min/max height of the world
    pub height: Range<f32>,
    /// How deep is the dirt generated (distance to air)
    pub dirt_height: usize,
    /// Height range in which grass and dirt are generated
    pub dirt_range: Range<isize>,
}

impl Default for WorldGen {
    fn default() -> Self {
        WorldGen {
            base: NoiseParam {
                octaves: 6,
                frequency: 0.02,
                lacunarity: PI * 2.0 / 3.0,
                persistence: 1.0,
                attenuation: 2.0,
            },
            base_limit: -f32::INFINITY..0.5,
            base_strength: 0.4,
            cave_limit: -0.1..0.1,
            height: MIN_HEIGHT as _..MAX_HEIGHT as _,
            dirt_height: DIRT_HEIGHT,
            dirt_range: MIN_HEIGHT / 2..MAX_HEIGHT / 2,
        }
    }
}

/// Generate a new chunk at this position with the given noise configuration.
pub fn generate_chunk(pos: IVec3, gen: &WorldGen) -> Chunk {
    if pos.y > (gen.height.end / Chunk::SIZE as f32).ceil() as i32 {
        // air
        return Chunk::new(BlockId(0));
    } else if pos.y < ((gen.height.start - 1.0) / Chunk::SIZE as f32).floor() as i32 {
        // stone
        return Chunk::new(BlockId(1));
    }

    let mut chunk = Chunk::new(BlockId(0));

    let b_pos = pos * Chunk::SIZE as i32;

    let solid = RigedSimplex::new(&gen.base)
        .map(|p, v| gen.base_strength * v + gen.height.lerp_inv(p.y as _));

    for_uvec3(UVec3::ZERO, Chunk::MAX, |p| {
        let gp = p.as_ivec3() + b_pos;

        if gen.base_limit.contains(&solid.get(gp)) {
            // Dirt and grass
            if gen.dirt_range.contains(&(gp.y as isize)) {
                if !gen.base_limit.contains(&solid.get(gp + IVec3::Y)) {
                    chunk[p] = BlockId(3);
                    return;
                } else {
                    for i in 2..=gen.dirt_height as i32 {
                        if !gen.base_limit.contains(&solid.get(gp + i * IVec3::Y)) {
                            chunk[p] = BlockId(2);
                            return;
                        }
                    }
                }
            }

            // Or Stone...
            chunk[p] = BlockId(1);
        }
    });
    chunk
}

/// 3D Noise
trait Noise3D: Sized {
    fn get(&self, p: IVec3) -> f32;

    fn map<F: Fn(IVec3, f32) -> f32>(self, f: F) -> Map<Self, F> {
        Map { noise: self, f }
    }
    fn map_mut<F: FnMut(IVec3, f32) -> f32>(self, f: F) -> MapMut<Self, F> {
        MapMut {
            noise: self,
            f: RefCell::new(f),
        }
    }
    fn generate(self, start: IVec3, size: usize) -> Generated {
        Generated::new(self, start, size)
    }
}

/// Wrapper for a 3D noise
#[derive(Clone)]
struct RigedSimplex {
    inner: RidgedMulti<Simplex>,
}

impl RigedSimplex {
    fn new(param: &NoiseParam) -> Self {
        let inner = RidgedMulti::<Simplex>::new(0)
            .set_octaves(param.octaves)
            .set_frequency(param.frequency as _)
            .set_lacunarity(param.lacunarity as _)
            .set_persistence(param.persistence as _)
            .set_attenuation(param.attenuation as _);
        Self { inner }
    }
}

impl Noise3D for RigedSimplex {
    fn get(&self, p: IVec3) -> f32 {
        self.inner.get(p.as_dvec3().to_array()) as _
    }
}

/// Postprocesses the noise output with f
struct Map<N: Noise3D, F: Fn(IVec3, f32) -> f32> {
    noise: N,
    f: F,
}

impl<N: Noise3D, F: Fn(IVec3, f32) -> f32> Noise3D for Map<N, F> {
    fn get(&self, index: IVec3) -> f32 {
        (&self.f)(index, self.noise.get(index))
    }
}

/// Postprocesses the noise output with f
struct MapMut<N: Noise3D, F: FnMut(IVec3, f32) -> f32> {
    noise: N,
    /// Yes a little bit ugly...
    f: RefCell<F>,
}

impl<N: Noise3D, F: FnMut(IVec3, f32) -> f32> Noise3D for MapMut<N, F> {
    fn get(&self, index: IVec3) -> f32 {
        (self.f.borrow_mut())(index, self.noise.get(index))
    }
}

/// Pregenerates the noise value for a 3D cube
struct Generated {
    /// Data in yzx order
    data: Vec<f32>,
    start: IVec3,
    size: usize,
}

impl Generated {
    fn new(noise: impl Noise3D, start: IVec3, size: usize) -> Self {
        let mut data = Vec::with_capacity(size * size * size);
        for_uvec3(UVec3::ZERO, UVec3::splat(size as _), |p| {
            let p = start + p.as_ivec3();
            data.push(noise.get([p.y, p.z, p.x].into()) as _);
        });
        Self { data, start, size }
    }
}

impl Noise3D for Generated {
    fn get(&self, p: IVec3) -> f32 {
        let p = p - self.start;
        assert!(p.min_element() >= 0 && p.max_element() < self.size as i32);
        let i = p.y as usize + self.size * (p.z as usize + self.size * p.x as usize);
        self.data[i]
    }
}
