use bevy::math::IVec3;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::hashbrown::HashMap;
use futures_lite::future;

use crate::chunk::Chunk;
use crate::generation::generate_chunk;
use crate::player::{PlayerController, PlayerSettings};
use crate::{AppState, BlockMat};

enum ChunkData {
    Generating,
    Visible(Chunk),
}

#[derive(Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, ChunkData>,
}

impl VoxelWorld {
    pub fn chunk_pos(p: Vec3) -> IVec3 {
        p.as_ivec3() / Chunk::SIZE as i32
    }
    pub fn world_pos(p: IVec3) -> Vec3 {
        p.as_vec3() * Chunk::SIZE as f32
    }
}

#[derive(Debug, Default, Clone, Copy, Component, PartialEq, Eq)]
struct ChunkMesh(IVec3);

struct GeneratedChunk(IVec3, Chunk, Mesh);

fn init_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    thread_pool: Res<AsyncComputeTaskPool>,
    query: Query<&Transform, With<PlayerController>>,
) {
    let player_transform = query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);

    let dist = settings.view_distance as i32;
    for x in -dist..=dist {
        for z in -dist..=dist {
            for y in -dist..=dist {
                let coord = center + IVec3::new(x, y, z);
                world.chunks.entry(coord).or_insert_with(|| {
                    let task = thread_pool.spawn(async move {
                        let chunk = generate_chunk(coord);
                        let mesh = chunk.mesh();
                        GeneratedChunk(coord, chunk, mesh)
                    });
                    cmds.spawn().insert(task);
                    info!("generate {coord}");
                    ChunkData::Generating
                });
            }
        }
    }
}

fn handle_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_mat: Res<BlockMat>,
    mut query: Query<(Entity, &mut Task<GeneratedChunk>)>,
) {
    for (entity, mut task) in query.iter_mut() {
        if let Some(GeneratedChunk(coord, chunk, mesh)) =
            future::block_on(future::poll_once(&mut *task))
        {
            info!("drawing {coord}");
            cmds.entity(entity)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(mesh),
                    material: block_mat.0.clone(),
                    transform: Transform::from_translation(VoxelWorld::world_pos(coord)),
                    ..default()
                })
                .insert(ChunkMesh(coord))
                .remove::<Task<GeneratedChunk>>();
            let previous = world.chunks.insert(coord, ChunkData::Visible(chunk));
            assert!(matches!(previous, Some(ChunkData::Generating)));
        }
    }
}

fn despawn_chunks(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    query: Query<&Transform, With<PlayerController>>,
    chunk_query: Query<(Entity, &ChunkMesh)>,
) {
    let player_transform = query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);

    let dist = settings.view_distance as u32;

    for (entity, ChunkMesh(coord)) in chunk_query.iter() {
        if distance(center - *coord) > dist + 1 {
            info!("despawn {coord}");
            cmds.entity(entity).despawn();
            world.chunks.remove(coord);
        }
    }
}

fn distance(p: IVec3) -> u32 {
    p.max_element().abs().max(p.min_element().abs()) as _
}

#[derive(Default)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorld>().add_system_set(
            SystemSet::on_update(AppState::Running)
                .with_system(init_generation)
                .with_system(handle_generation)
                .with_system(despawn_chunks),
        );
    }
}
