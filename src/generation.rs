use std::ops::Range;

use bevy::prelude::*;
use simdnoise::NoiseBuilder;

use crate::block::BlockId;
use crate::chunk::Chunk;
use crate::util::RangeExt;

const MIN_HEIGHT: f32 = -128.0;
const MAX_HEIGHT: f32 = 128.0;

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
        info!("{pos} -> {b_pos}");
        let (base, min, max) = NoiseBuilder::ridge_3d_offset(
            b_pos.x,
            Chunk::SIZE as _,
            b_pos.y,
            Chunk::SIZE as _,
            b_pos.z,
            Chunk::SIZE as _,
        )
        .with_freq(noise.freq)
        .with_lacunarity(noise.lacunarity)
        .with_gain(noise.gain)
        .with_octaves(noise.octaves)
        .generate();
        info!("[{min:.3}, {max:.3}]");

        for x in 0..Chunk::SIZE {
            for z in 0..Chunk::SIZE {
                for y in 0..Chunk::SIZE {
                    let gy = y as i32 + pos.y * Chunk::SIZE as i32;

                    // The higher the lower the propability for stone
                    let height = 1.0 - noise.height.lerp_inv(gy as _);
                    let limit = noise.limits.lerp(height);

                    let filled = base[x + y * Chunk::SIZE + z * Chunk::SIZE * Chunk::SIZE] < limit;
                    if filled {
                        chunk[UVec3::new(x as _, y as _, z as _)] = BlockId(1);
                    } else {
                        // Dirt and grass
                        if y > 0 && chunk[UVec3::new(x as _, y as u32 - 1, z as _)] != BlockId(0) {
                            chunk[UVec3::new(x as _, y as u32 - 1, z as _)] = BlockId(3);
                            if y > 1
                                && chunk[UVec3::new(x as _, y as u32 - 2, z as _)] != BlockId(0)
                            {
                                chunk[UVec3::new(x as _, y as u32 - 2, z as _)] = BlockId(2);
                            }
                        }
                    }
                }
            }
        }
    }
    chunk
}
