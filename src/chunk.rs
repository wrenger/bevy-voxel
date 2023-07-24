use std::fmt;
use std::ops::{Index, IndexMut};

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::utils::HashMap;

use crate::block::{Block, BlockId, blocks};
use crate::util::{for_uvec3, Direction};

/// Each chunk contains a number of blocks.
/// A single mesh covering all the blocks is generated for every chunk.
#[derive(Clone)]
pub struct Chunk {
    /// They are stored in the order: Y, Z, X (in -> out)
    blocks: Box<[[[BlockId; Chunk::SIZE]; Chunk::SIZE]; Chunk::SIZE]>,
}

impl Chunk {
    pub const SIZE: usize = 32;
    pub const MAX: UVec3 = UVec3::splat(Self::SIZE as u32);

    pub fn new(block: BlockId) -> Self {
        Self {
            blocks: Box::new([[[block; Chunk::SIZE]; Chunk::SIZE]; Chunk::SIZE]),
        }
    }

    fn occupied(&self, pos: UVec3, blocks: &HashMap<BlockId, Block>) -> bool {
        debug_assert!(pos.cmplt(Self::MAX).all(), "{pos:?}");
        blocks[&self[pos]].opaque
    }

    pub fn border(&self, dir: Direction, blocks: &HashMap<BlockId, Block>) -> Border {
        let mut border = Border::new();
        for y in 0..Self::SIZE as u32 {
            for x in 0..Self::SIZE as u32 {
                let pos = Self::from_surface(dir, UVec2::new(x, y));
                if self.occupied(pos, blocks) {
                    border.set_occupied(UVec2::new(x, y));
                }
            }
        }
        border
    }

    /// Computes a single mesh over all blocks.
    /// Not visible faces are excluded.
    pub fn mesh(&self, neighbors: [Border; 6]) -> Mesh {
        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);
        let mut indices = Vec::new();

        let blocks = blocks().read().unwrap();

        for_uvec3(UVec3::ZERO, Self::MAX, |pos| {
            let occupied = Direction::all().map(|d| {
                let p = pos.as_ivec3() + IVec3::from(d);
                if p.cmpge(IVec3::ZERO).all() && p.cmplt(Self::MAX.as_ivec3()).all() {
                    self.occupied(p.as_uvec3(), &blocks)
                } else {
                    // Check neighbors if out of bounds
                    let p = (p + Self::MAX.as_ivec3()).as_uvec3() % Self::MAX;
                    let p2 = Self::to_surface(d.inverse(), p);
                    neighbors[d as usize].occupied(p2)
                }
            });

            if !occupied.iter().all(|b| *b) {
                let block = &blocks[&self[pos]];
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
        });

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }

    fn from_surface(d: Direction, v: UVec2) -> UVec3 {
        let center = (Self::MAX.as_vec3() - 1.0) / 2.0;
        let pos = Vec3::new(v.x as _, v.y as _, 0.0);
        ((Quat::from(d) * (pos - center)) + center)
            .round()
            .as_uvec3()
    }

    fn to_surface(d: Direction, p: UVec3) -> UVec2 {
        let center = (Self::MAX.as_vec3() - 1.0) / 2.0;
        let pos = p.as_vec3() - center;
        let pos = Quat::from(d).inverse() * pos + center;
        let pos = pos.round().as_uvec3();
        debug_assert!(pos.z == 0);
        pos.truncate()
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

#[derive(Clone, Copy)]
pub struct Border([u8; Chunk::SIZE * Chunk::SIZE / 8]);

impl Border {
    pub fn new() -> Self {
        Self([0; Chunk::SIZE * Chunk::SIZE / 8])
    }
    fn occupied(self, p: UVec2) -> bool {
        debug_assert!(p.cmplt(Chunk::SIZE as u32 * UVec2::ONE).all());
        let i = p.x + p.y * Chunk::SIZE as u32;
        let j = i / 8;
        let k = i % 8;
        self.0[j as usize] & 1 << k != 0
    }
    fn set_occupied(&mut self, p: UVec2) {
        debug_assert!(p.cmplt(Chunk::SIZE as u32 * UVec2::ONE).all());
        let i = p.x + p.y * Chunk::SIZE as u32;
        let j = i / 8;
        let k = i % 8;
        self.0[j as usize] |= 1 << k;
    }
}

impl fmt::Debug for Border {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Border(")?;
        for row in self.0.chunks(Chunk::SIZE / 8).rev() {
            write!(f, "   ")?;
            for v in row {
                write!(f, " {:08b}", v.reverse_bits())?;
            }
            writeln!(f)?;
        }
        writeln!(f, ")")?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use bevy::prelude::*;
    use bevy::utils::HashMap;

    use super::Chunk;
    use crate::block::{Block, BlockId};
    use crate::util::Direction;

    #[test]
    fn border() {
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                cubes: Vec::new(),
                opaque: false,
            },
        );
        blocks.insert(
            BlockId(1),
            Block {
                cubes: Vec::new(),
                opaque: true,
            },
        );

        let mut chunk = Chunk::new(BlockId(0));
        let pos = [
            UVec3::new(0, 1, 2),
            UVec3::new(3, 0, 4),
            UVec3::new(5, 6, 0),
            UVec3::new(31, 31 - 7, 31 - 8),
            UVec3::new(31 - 9, 31, 31 - 10),
            UVec3::new(31 - 11, 31 - 12, 31),
        ];

        for p in &pos {
            chunk[*p] = BlockId(1);
        }

        for d in Direction::all() {
            let border = chunk.border(d, &blocks);
            let p = pos[d as usize];
            let p2 = Chunk::to_surface(d, p);

            println!("{d:?}: p={p} p2={p2}");
            println!("{border:?}");

            for y in 0..Chunk::SIZE as u32 {
                for x in 0..Chunk::SIZE as u32 {
                    let p = UVec2::new(x, y);
                    if !(border.occupied(p) == (p == p2)) {
                        eprintln!("invalid {p}");
                    }
                }
            }
        }
    }
}
