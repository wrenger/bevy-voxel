use std::ops::Range;

use bevy::prelude::*;
use simdnoise::NoiseBuilder;

use crate::block::BlockId;
use crate::chunk::Chunk;
use crate::util::RangeExt;

const MIN_HEIGHT: f32 = -128.0;
const MAX_HEIGHT: f32 = 128.0;
const BORDER: usize = 2;

#[derive(Debug, Clone)]
pub struct Noise {
    pub freq: f32,
    pub lacunarity: f32,
    pub gain: f32,
    pub octaves: u8,
    pub limits: Range<f32>,
    pub height: Range<f32>,
}

impl Default for Noise {
    fn default() -> Self {
        Noise {
            freq: 0.05,
            lacunarity: 0.65,
            gain: 2.0,
            octaves: 4,
            limits: 3.5..4.0,
            height: MIN_HEIGHT..MAX_HEIGHT,
        }
    }
}

/// Generate a new chunk at this position with the given noise configuration.
pub fn generate_chunk(pos: IVec3, noise: &Noise) -> Chunk {
    let mut chunk = Chunk::new();
    if pos.y > (noise.height.end / Chunk::SIZE as f32).ceil() as i32 {
        // air
    } else if pos.y < ((noise.height.start - 1.0) / Chunk::SIZE as f32).floor() as i32 {
        // stone
        chunk.fill(BlockId(1), UVec3::ZERO, Chunk::MAX - 1);
    } else {
        let b_pos = pos.as_vec3() * Chunk::SIZE as f32;
        let (base, _, _) = NoiseBuilder::ridge_3d_offset(
            b_pos.x - BORDER as f32,
            Chunk::SIZE + 2 * BORDER,
            b_pos.y - BORDER as f32,
            Chunk::SIZE + 2 * BORDER,
            b_pos.z - BORDER as f32,
            Chunk::SIZE + 2 * BORDER,
        )
        .with_freq(noise.freq)
        .with_lacunarity(noise.lacunarity)
        .with_gain(noise.gain)
        .with_octaves(noise.octaves)
        .generate();

        let idx = |x: isize, y: isize, z: isize| {
            debug_assert!(
                x >= -(BORDER as isize)
                    && y >= -(BORDER as isize)
                    && z >= -(BORDER as isize)
                    && x < (Chunk::SIZE + BORDER) as isize
                    && y < (Chunk::SIZE + BORDER) as isize
                    && z < (Chunk::SIZE + BORDER) as isize
            );
            (BORDER as isize + x) as usize
                + (BORDER as isize + y) as usize * (Chunk::SIZE + BORDER * 2)
                + (BORDER as isize + z) as usize
                    * (Chunk::SIZE + BORDER * 2)
                    * (Chunk::SIZE + BORDER * 2)
        };

        for x in 0..Chunk::SIZE as isize {
            for z in 0..Chunk::SIZE as isize {
                for y in 0..Chunk::SIZE as isize {
                    let propability = |y: isize| {
                        let gy = y as i32 + pos.y * Chunk::SIZE as i32;
                        // The higher the lower the propability for stone
                        noise.limits.lerp(1.0 - noise.height.lerp_inv(gy as _))
                    };

                    if base[idx(x, y, z)] < propability(y) {
                        // Dirt and grass
                        if base[idx(x, y + 1, z)] >= propability(y + 1) {
                            chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(3);
                        } else if base[idx(x, y + 2, z)] >= propability(y + 2) {
                            chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(2);
                        } else {
                            // Stone
                            chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(1);
                        }
                    }
                }
            }
        }
    }
    chunk
}
