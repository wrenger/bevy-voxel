use std::ops::Range;

use bevy::prelude::*;
use simdnoise::NoiseBuilder;

use crate::block::BlockId;
use crate::chunk::Chunk;
use crate::util::RangeExt;

const MIN_HEIGHT: isize = -128;
const MAX_HEIGHT: isize = 128;
const DIRT_HEIGHT: usize = 2;

#[derive(Debug, Resource, Clone)]
pub struct WorldGen {
    pub freq: f32,
    pub lacunarity: f32,
    pub gain: f32,
    pub octaves: u8,
    pub limits: Range<f32>,
    pub height: Range<f32>,
    pub dirt_height: usize,
    pub dirt_range: Range<isize>,
}

impl Default for WorldGen {
    fn default() -> Self {
        WorldGen {
            freq: 0.05,
            lacunarity: 0.65,
            gain: 2.0,
            octaves: 4,
            limits: 3.5..4.0,
            height: MIN_HEIGHT as _..MAX_HEIGHT as _,
            dirt_height: DIRT_HEIGHT,
            dirt_range: MIN_HEIGHT / 2..MAX_HEIGHT / 2,
        }
    }
}

/// Generate a new chunk at this position with the given noise configuration.
pub fn generate_chunk(pos: IVec3, gen: &WorldGen) -> Chunk {
    let mut chunk = Chunk::new();
    if pos.y > (gen.height.end / Chunk::SIZE as f32).ceil() as i32 {
        // air
    } else if pos.y < ((gen.height.start - 1.0) / Chunk::SIZE as f32).floor() as i32 {
        // stone
        chunk.fill(BlockId(1), UVec3::ZERO, Chunk::MAX - 1);
    } else {
        let border = gen.dirt_height;

        let b_pos = pos.as_vec3() * Chunk::SIZE as f32;
        let (base, _, _) = NoiseBuilder::ridge_3d_offset(
            b_pos.x - border as f32,
            Chunk::SIZE + 2 * border,
            b_pos.y - border as f32,
            Chunk::SIZE + 2 * border,
            b_pos.z - border as f32,
            Chunk::SIZE + 2 * border,
        )
        .with_freq(gen.freq)
        .with_lacunarity(gen.lacunarity)
        .with_gain(gen.gain)
        .with_octaves(gen.octaves)
        .generate();

        let idx = |x: isize, y: isize, z: isize| {
            debug_assert!(
                x >= -(border as isize)
                    && y >= -(border as isize)
                    && z >= -(border as isize)
                    && x < (Chunk::SIZE + border) as isize
                    && y < (Chunk::SIZE + border) as isize
                    && z < (Chunk::SIZE + border) as isize
            );
            (border as isize + x) as usize
                + (border as isize + y) as usize * (Chunk::SIZE + border * 2)
                + (border as isize + z) as usize
                    * (Chunk::SIZE + border * 2)
                    * (Chunk::SIZE + border * 2)
        };

        for x in 0..Chunk::SIZE as isize {
            for z in 0..Chunk::SIZE as isize {
                'block: for y in 0..Chunk::SIZE as isize {
                    let is_solid = |x: isize, y: isize, z: isize| {
                        let gy = y as i32 + pos.y * Chunk::SIZE as i32;
                        // The higher the lower the propability for stone
                        let propability = gen.limits.lerp(1.0 - gen.height.lerp_inv(gy as _));
                        base[idx(x, y, z)] < propability
                    };

                    let gy = y + pos.y as isize * Chunk::SIZE as isize;

                    if is_solid(x, y, z) {
                        // Dirt and grass
                        if gen.dirt_range.contains(&gy) {
                            if !is_solid(x, y + 1, z) {
                                chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(3);
                                continue 'block;
                            } else {
                                for i in 2..=gen.dirt_height as isize {
                                    if !is_solid(x, y + i, z) {
                                        chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(2);
                                        continue 'block;
                                    }
                                }
                            }
                        }

                        // Or Stone...
                        chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(1);
                    }
                }
            }
        }
    }
    chunk
}
