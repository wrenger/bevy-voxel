use bevy::prelude::*;
use std::f32::consts;

use crate::block::BlockId;
use crate::chunk::Chunk;

const DIRT_HEIGHT: f32 = 3.0;

const MIN_HEIGHT: f32 = -16.0;
const MAX_HEIGHT: f32 = 0.0;

pub fn generate_chunk(pos: IVec3) -> Chunk {
    let mut chunk = Chunk::new();
    if pos.y > (MAX_HEIGHT / Chunk::SIZE as f32).ceil() as i32 {
    } else if pos.y < ((MIN_HEIGHT - DIRT_HEIGHT) / Chunk::SIZE as f32).floor() as i32 {
        chunk.fill(BlockId(1), UVec3::ZERO, Chunk::MAX - 1);
    } else {
        for x in 0..Chunk::SIZE as u32 {
            for z in 0..Chunk::SIZE as u32 {
                let gx = x as i32 + pos.x * Chunk::SIZE as i32;
                let gz = z as i32 + pos.z * Chunk::SIZE as i32;

                let h = height(gx, gz);

                for y in 0..Chunk::SIZE as u32 {
                    let gy = y as i32 + pos.y * Chunk::SIZE as i32;
                    let p = UVec3::new(x, y, z);
                    if gy < h - DIRT_HEIGHT as i32 {
                        chunk[p] = BlockId(1);
                    } else if gy < h {
                        chunk[p] = BlockId(2);
                    } else if gy == h {
                        chunk[p] = BlockId(3);
                    }
                }
            }
        }
    }
    chunk
}

fn height(x: i32, z: i32) -> i32 {
    let rx = (x as f32 / Chunk::SIZE as f32 * 2.0 * consts::PI).sin() / 2.0 + 1.0;
    let rz = (z as f32 / Chunk::SIZE as f32 * 2.0 * consts::PI).sin() / 2.0 + 1.0;

    (MIN_HEIGHT + (rx * rz * (MAX_HEIGHT - MIN_HEIGHT))) as i32
}
