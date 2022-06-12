use std::ops::{Index, IndexMut};
use std::sync::RwLockReadGuard;

use bevy::math::const_uvec3;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::utils::HashMap;

use crate::block::{Block, BlockId, BLOCKS};
use crate::util::Direction;

pub struct Chunk {
    /// They are stored in the order: Y, Z, X (in -> out)
    blocks: Box<[[[BlockId; Chunk::SIZE]; Chunk::SIZE]; Chunk::SIZE]>,
}

impl Chunk {
    pub const SIZE: usize = 32;
    pub const MAX: UVec3 = const_uvec3!([Self::SIZE as u32; 3]);

    pub fn new() -> Self {
        Self {
            blocks: Box::new([[[BlockId(0); Chunk::SIZE]; Chunk::SIZE]; Chunk::SIZE]),
        }
    }

    pub fn fill(&mut self, block: BlockId, from: UVec3, to: UVec3) {
        debug_assert!(
            (from.x <= to.x && to.x < Self::SIZE as u32)
                && (from.y <= to.y && to.y < Self::SIZE as u32)
        );
        for x in from.y..=to.y {
            for z in from.z..=to.z {
                for y in from.x..=to.x {
                    self[UVec3::new(x, y, z)] = block;
                }
            }
        }
    }

    fn occupied(
        &self,
        pos: UVec3,
        dir: Direction,
        blocks: &RwLockReadGuard<'_, HashMap<BlockId, Block>>,
    ) -> bool {
        let p = pos.as_ivec3() + IVec3::from(dir);
        p.cmpge(IVec3::ZERO).all()
            && p.cmplt(Self::MAX.as_ivec3()).all()
            && blocks[&self[p.as_uvec3()]].opaque
    }

    pub fn mesh(&self) -> Mesh {
        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);
        let mut indices = Vec::new();

        let blocks = BLOCKS.read().unwrap();

        for x in 0..Self::SIZE {
            for z in 0..Self::SIZE {
                for y in 0..Self::SIZE {
                    let pos = UVec3::new(x as _, y as _, z as _);
                    let id = self[pos];
                    let occupied = Direction::all().map(|d| self.occupied(pos, d, &blocks));

                    let block = &blocks[&id];
                    if !occupied.iter().all(|b| *b) {
                        for cube in &block.cubes {
                            cube.mesh(
                                pos.as_vec3(),
                                occupied,
                                &mut indices,
                                &mut positions,
                                &mut normals,
                                &mut uvs,
                            );
                        }
                    }
                }
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }
}

impl Index<UVec3> for Chunk {
    type Output = BlockId;

    fn index(&self, index: UVec3) -> &Self::Output {
        &self.blocks[index.x as usize][index.z as usize][index.y as usize]
    }
}

impl IndexMut<UVec3> for Chunk {
    fn index_mut(&mut self, index: UVec3) -> &mut Self::Output {
        &mut self.blocks[index.x as usize][index.z as usize][index.y as usize]
    }
}
