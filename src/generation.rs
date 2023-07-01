use std::f32::consts::PI;
use std::ops::{Index, Range};

use bevy::prelude::*;
use noise::{MultiFractal, NoiseFn, Perlin, RidgedMulti};

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
            cave_limit: -0.1..0.1,
            base_strength: 0.4,
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
    let border = gen.dirt_height;

    let b_pos = pos * Chunk::SIZE as i32;
    let noise_size = Chunk::SIZE + 2 * border;

    let mut solid = Noise::generate(&gen.base, b_pos - IVec3::splat(border as _), noise_size);
    solid.apply(|p, v| gen.base_strength * v + gen.height.lerp_inv(p.y as _));

    for_uvec3(UVec3::ZERO, Chunk::MAX, |p| {
        let gp = p.as_ivec3() + b_pos;

        if gen.base_limit.contains(&solid[gp]) {
            // Dirt and grass
            if gen.dirt_range.contains(&(gp.y as isize)) {
                if !gen.base_limit.contains(&solid[gp + IVec3::Y]) {
                    chunk[p] = BlockId(3);
                    return;
                } else {
                    for i in 2..=gen.dirt_height as i32 {
                        if !gen.base_limit.contains(&solid[gp + i * IVec3::Y]) {
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

#[derive(Clone)]
struct Noise {
    start: IVec3,
    size: usize,
    data: Vec<f32>,
}

impl Noise {
    fn generate(param: &NoiseParam, start: IVec3, size: usize) -> Self {
        let noise = RidgedMulti::<Perlin>::new(0)
            .set_octaves(param.octaves)
            .set_frequency(param.frequency as _)
            .set_lacunarity(param.lacunarity as _)
            .set_persistence(param.persistence as _)
            .set_attenuation(param.attenuation as _);
        let mut data = Vec::with_capacity(size * size * size);
        for_uvec3(UVec3::ZERO, UVec3::splat(size as _), |p| {
            let p = start.as_dvec3() + p.as_dvec3();
            let v = noise.get([p.y, p.z, p.x]) as f32;
            assert!((-1.0..=1.0).contains(&v));
            data.push(v);
        });
        Self { start, size, data }
    }

    fn apply(&mut self, mut f: impl FnMut(IVec3, f32) -> f32) {
        for (i, v) in self.data.iter_mut().enumerate() {
            let size = self.size as i32;
            let i = i as i32;
            let di = i / size;
            let vec = self.start + IVec3::new(di % size, i % size, (di / size) % size);
            *v = f(vec, *v);
        }
    }
}

impl Index<IVec3> for Noise {
    type Output = f32;

    fn index(&self, index: IVec3) -> &Self::Output {
        let offset = index - self.start;
        assert!(offset.min_element() >= 0 && offset.max_element() < self.size as i32);
        let i = offset.y as usize + self.size * (offset.z as usize + self.size * offset.x as usize);
        &self.data[i]
    }
}
