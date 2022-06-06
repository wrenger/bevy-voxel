use std::ops::{Index, IndexMut};

use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::math::const_uvec3;
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::once_cell::sync::OnceCell;
use serde::Deserialize;

use crate::textures::{TextureMap, TextureMapId};
use crate::util::Direction;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u16);

pub static BLOCK_HANDLES: OnceCell<Vec<Handle<Block>>> = OnceCell::new();

impl BlockId {
    pub fn handle(self) -> Handle<Block> {
        BLOCK_HANDLES.get().expect("Blocks not initialized")[self.0 as usize].clone_weak()
    }
}

/// Block occupying a specific coordinate.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "fd6772fe-c8b7-4e89-b1f8-4af6faa57626"]
pub struct Block {
    pub opaque: bool,
    pub cubes: Vec<Cube>,
}

impl Block {
    pub fn mesh(&self) -> Mesh {
        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);
        let mut indices = Vec::new();

        for cube in &self.cubes {
            cube.mesh(
                Vec3::ZERO,
                [false; 6],
                &mut indices,
                &mut positions,
                &mut normals,
                &mut uvs,
            );
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(indices)));
        mesh
    }
}

#[derive(Debug, Clone)]
pub struct Cube {
    pub min: UVec3,
    pub max: UVec3,
    pub faces: [Face; 6],
}

impl Cube {
    pub const MAX: UVec3 = const_uvec3!([16; 3]);

    pub fn minf(&self) -> Vec3 {
        self.min.as_vec3() / Self::MAX.as_vec3()
    }
    pub fn maxf(&self) -> Vec3 {
        self.max.as_vec3() / Self::MAX.as_vec3()
    }

    pub fn mesh(
        &self,
        pos: Vec3,
        occupied: [bool; 6],
        indices: &mut Vec<u32>,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        let r_p = &[
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
        ];

        let r_uvs = &[
            // -x
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
        ];

        for d in Direction::all() {
            let face = &self.faces[d as usize];
            if !(face.cull == Some(d) && occupied[d as usize]) {
                let rot = Quat::from(d);
                for p in r_p {
                    // Rotate and normalize to (0, 1)
                    let p = (rot * *p) + Vec3::new(0.5, 0.5, 0.5);
                    // Scale to cube size
                    let p = self.minf() + p * (self.maxf() - self.minf());
                    let p = p + pos;
                    positions.push(p.into());
                }

                normals.extend_from_slice(&[Vec3::from(d).into(); 4]);

                let uv = TextureMap::get().uv(face.texture);
                // TODO: Scale to cube size
                uvs.extend_from_slice(&[
                    (uv.0 + r_uvs[0] * (uv.1 - uv.0)).into(),
                    (uv.0 + r_uvs[1] * (uv.1 - uv.0)).into(),
                    (uv.0 + r_uvs[2] * (uv.1 - uv.0)).into(),
                    (uv.0 + r_uvs[3] * (uv.1 - uv.0)).into(),
                ]);

                let j = indices.len() as u32 / 6 * 4;
                indices.extend_from_slice(&[j, j + 1, j + 2, j, j + 2, j + 3]);
            }
        }
    }
}

impl Index<Direction> for Cube {
    type Output = Face;

    fn index(&self, index: Direction) -> &Self::Output {
        &self.faces[index as usize]
    }
}

impl IndexMut<Direction> for Cube {
    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        &mut self.faces[index as usize]
    }
}

#[derive(Debug, Clone)]
pub struct Face {
    pub texture: TextureMapId,
    pub cull: Option<Direction>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct BlockData {
    cubes: Vec<CubeData>,
    opaque: bool,
}

#[derive(Debug, Deserialize)]
struct CubeData {
    #[serde(default)]
    min: UVec3,
    #[serde(default = "cube_max")]
    max: UVec3,
    faces: [FaceData; 6],
}

fn cube_max() -> UVec3 {
    Cube::MAX
}

#[derive(Debug, Deserialize)]
struct FaceData {
    texture: String,
    cull: Option<Direction>,
}

#[derive(Default)]
pub struct BlockLoader;

impl AssetLoader for BlockLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let block_data: BlockData = serde_json::from_slice(bytes)?;

            let texture_map = TextureMap::get();

            let block = Block {
                opaque: block_data.opaque,
                cubes: block_data
                    .cubes
                    .into_iter()
                    .map(|c| Cube {
                        min: c.min,
                        max: c.max,
                        faces: [
                            Face {
                                texture: texture_map.id(&c.faces[0].texture),
                                cull: c.faces[0].cull,
                            },
                            Face {
                                texture: texture_map.id(&c.faces[1].texture),
                                cull: c.faces[1].cull,
                            },
                            Face {
                                texture: texture_map.id(&c.faces[2].texture),
                                cull: c.faces[2].cull,
                            },
                            Face {
                                texture: texture_map.id(&c.faces[3].texture),
                                cull: c.faces[3].cull,
                            },
                            Face {
                                texture: texture_map.id(&c.faces[4].texture),
                                cull: c.faces[4].cull,
                            },
                            Face {
                                texture: texture_map.id(&c.faces[5].texture),
                                cull: c.faces[5].cull,
                            },
                        ],
                    })
                    .collect(),
            };

            load_context.set_default_asset(LoadedAsset::new(block));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["block"]
    }
}