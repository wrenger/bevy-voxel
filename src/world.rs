use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::hashbrown::HashMap;
use futures_lite::future;

use crate::block::blocks;
use crate::chunk::{Border, Chunk};
use crate::generation::{generate_chunk, WorldGen};
use crate::player::{PlayerController, PlayerSettings};
use crate::util::Direction;
use crate::{AppState, BlockMat};

/// The world, consisting of smaller chunks
#[derive(Default, Resource)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, Entity>,
}

impl VoxelWorld {
    pub fn chunk_pos(p: Vec3) -> IVec3 {
        ((p - (Chunk::SIZE as f32 / 2.0)) / Chunk::SIZE as f32)
            .round()
            .as_ivec3()
    }
    pub fn world_pos(p: IVec3) -> Vec3 {
        p.as_vec3() * Chunk::SIZE as f32
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct ChunkPos(IVec3);

#[derive(Component)]
struct ChunkData(Arc<Chunk>);

#[derive(Component)]
struct Generating(Task<Chunk>);

#[derive(Component, Debug)]
struct MissingNeighbors(usize);

#[derive(Component)]
struct RequiresMesh;

#[derive(Component)]
struct Meshing(Task<Mesh>);

fn init_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    noise: Res<WorldGen>,
    query: Query<&Transform, With<PlayerController>>,
) {
    let player_transform = query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);

    let dist = settings.view_distance as i32 + 1;

    let thread_pool = AsyncComputeTaskPool::get();

    for d in 0..dist {
        for x in -dist..=dist {
            for z in -dist..=dist {
                for y in -dist..=dist {
                    let off = IVec3::new(x, y, z);
                    if distance(off) != d as u32 {
                        continue;
                    }
                    let pos = center + off;
                    world.chunks.entry(pos).or_insert_with(|| {
                        let noise = noise.clone();
                        let task = thread_pool.spawn(async move { generate_chunk(pos, &noise) });
                        let entity = cmds.spawn((ChunkPos(pos), Generating(task))).id();
                        entity
                    });
                }
            }
        }
    }
}

fn handle_generation(
    mut cmds: Commands,
    world: Res<VoxelWorld>,
    mut query: Query<(Entity, &ChunkPos, &mut Generating)>,
    mut neighbors: Query<&mut MissingNeighbors>,
) {
    for (entity, ChunkPos(pos), mut task) in query.iter_mut() {
        if let Some(chunk) = future::block_on(future::poll_once(&mut task.0)) {
            let mut surrounded = Vec::with_capacity(6);
            if let Some(mut cmds) = cmds.get_entity(entity) {
                let mut missing = 6;

                for d in Direction::all() {
                    if let Some(entity) = world.chunks.get(&(*pos + IVec3::from(d))) {
                        missing -= 1;
                        if let Ok(mut missing) = neighbors.get_mut(*entity) {
                            if missing.0 > 1 {
                                missing.0 -= 1;
                            } else {
                                surrounded.push(*entity);
                            }
                        }
                    }
                }

                if missing > 0 {
                    cmds.insert((MissingNeighbors(missing), ChunkData(Arc::new(chunk))))
                        .remove::<Generating>();
                } else {
                    cmds.insert((RequiresMesh, ChunkData(Arc::new(chunk))))
                        .remove::<Generating>();
                }
            }
            for entity in surrounded {
                cmds.get_entity(entity).map(|mut c| {
                    c.insert(RequiresMesh).remove::<MissingNeighbors>();
                });
            }
        }
    }
}

fn init_mesh(
    mut cmds: Commands,
    world: Res<VoxelWorld>,
    settings: Res<PlayerSettings>,
    player_query: Query<&Transform, With<PlayerController>>,
    query_mesh: Query<(Entity, &ChunkPos, &ChunkData, With<RequiresMesh>)>,
    query_data: Query<&ChunkData>,
) {
    let player_transform = player_query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);
    let dist = settings.view_distance as u32;
    let thread_pool = AsyncComputeTaskPool::get();

    query_mesh.for_each(|(entity, ChunkPos(pos), ChunkData(chunk), _)| {
        if distance(center - *pos) >= dist {
            return;
        }

        let blocks = blocks().read().unwrap();

        let mut borders = [Border::new(); 6];
        for d in Direction::all() {
            let Some(&entity) = world.chunks.get(&(*pos + IVec3::from(d))) else {
                return;
            };

            if let Ok(ChunkData(chunk)) = query_data.get(entity) {
                borders[d as usize] = chunk.border(d.inverse(), &blocks);
            } else {
                return;
            }
        }

        let chunk = chunk.clone();
        let task = thread_pool.spawn(async move { chunk.mesh(borders) });

        cmds.get_entity(entity).map(|mut cmds| {
            cmds.insert(Meshing(task)).remove::<RequiresMesh>();
        });
    });
}

fn handle_mesh(
    mut cmds: Commands,
    mut query: Query<(Entity, &ChunkPos, &mut Meshing)>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_mat: Res<BlockMat>,
) {
    for (entity, ChunkPos(pos), mut task) in query.iter_mut() {
        if let Some(mesh) = future::block_on(future::poll_once(&mut task.0)) {
            cmds.entity(entity)
                .insert((PbrBundle {
                    mesh: meshes.add(mesh),
                    material: block_mat.0.clone(),
                    transform: Transform::from_translation(VoxelWorld::world_pos(*pos)),
                    ..default()
                },))
                .remove::<Meshing>();
        }
    }
}

fn despawn_chunks(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    player: Query<&Transform, With<PlayerController>>,
    chunks: Query<(Entity, &ChunkPos)>,
) {
    let player_transform = player.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);

    let dist = settings.view_distance as u32;

    chunks.for_each(|(entity, ChunkPos(pos))| {
        if distance(center - *pos) > dist {
            cmds.entity(entity).despawn();
            world.chunks.remove(pos);
        }
    });
}

fn distance(p: IVec3) -> u32 {
    p.max_element().abs().max(p.min_element().abs()) as _
}

#[derive(Event)]
pub struct RegenerateEvent;

fn regenerate_chunks(
    mut events: EventReader<RegenerateEvent>,
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    chunks: Query<Entity, With<ChunkPos>>,
) {
    if !events.is_empty() {
        events.clear();

        warn!("Regenerate!");
        chunks.for_each(|entity| cmds.entity(entity).despawn());
        world.clear();
    }
}

#[derive(Component, Default)]
pub struct ChunkCenter;

fn move_chunk_center(
    player_query: Query<&Transform, With<PlayerController>>,
    mut query: Query<&mut Transform, (With<ChunkCenter>, Without<PlayerController>)>,
) {
    let player_transform = player_query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);
    query.for_each_mut(|mut t| {
        t.translation = VoxelWorld::world_pos(center) + Chunk::MAX.as_vec3() / 2.0
    });
}

#[derive(Default)]
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelWorld>()
            .add_event::<RegenerateEvent>()
            .add_systems(
                Update,
                (
                    init_generation,
                    handle_generation,
                    init_mesh,
                    handle_mesh,
                    despawn_chunks
                        .after(init_generation)
                        .after(handle_generation)
                        .after(init_mesh)
                        .after(handle_mesh),
                    regenerate_chunks.after(despawn_chunks),
                )
                    .run_if(in_state(AppState::Running)),
            )
            .add_systems(
                Update,
                move_chunk_center.run_if(in_state(AppState::Running)),
            );
    }
}
