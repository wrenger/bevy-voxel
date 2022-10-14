use std::mem::zeroed;

use bevy::math::IVec3;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::hashbrown::HashMap;
use futures_lite::future;

use crate::block::BLOCKS;
use crate::chunk::{Border, Chunk};
use crate::generation::{generate_chunk, Noise};
use crate::player::{PlayerController, PlayerSettings};
use crate::util::Direction;
use crate::{AppState, BlockMat};

enum ChunkData {
    Generating,
    Generated(Chunk),
    Visible(Chunk),
}

/// The world, consisting of smaller chunks
#[derive(Default)]
pub struct VoxelWorld {
    chunks: HashMap<IVec3, ChunkData>,
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

#[derive(Debug, Default, Clone, Copy, Component, PartialEq, Eq)]
struct GeneratedChunk(IVec3);

#[derive(Debug, Default, Clone, Copy, Component, PartialEq, Eq)]
struct VisibleChunk(IVec3);

#[derive(Component)]
struct ChunkResult(Task<(IVec3, Chunk)>);

fn init_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    noise: Res<Noise>,
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
                        let task = thread_pool.spawn(async move {
                            let chunk = generate_chunk(pos, &noise);
                            (pos, chunk)
                        });
                        cmds.spawn().insert(ChunkResult(task));
                        info!("generate {pos}");
                        ChunkData::Generating
                    });
                }
            }
        }
    }
}

fn handle_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    mut query: Query<(Entity, &mut ChunkResult)>,
) {
    for (entity, mut task) in query.iter_mut() {
        if let Some((pos, chunk)) = future::block_on(future::poll_once(&mut task.0)) {
            info!("generated {pos}");
            let previous = world.chunks.insert(pos, ChunkData::Generated(chunk));
            if let Some(ChunkData::Generating) = previous {
                cmds.entity(entity)
                    .insert(GeneratedChunk(pos))
                    .remove::<ChunkResult>();
            } else {
                warn!("Outdated chunk: {pos}");
            }
        }
    }
}

fn mesh_generation(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_mat: Res<BlockMat>,
    settings: Res<PlayerSettings>,
    player_query: Query<&Transform, With<PlayerController>>,
    query: Query<(Entity, &GeneratedChunk)>,
) {
    let player_transform = player_query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);
    let mut sorted = query.iter().collect::<Vec<_>>();
    sorted.sort_unstable_by_key(|(_, GeneratedChunk(p))| distance(*p - center));

    let dist = settings.view_distance as u32;

    'entities: for (entity, GeneratedChunk(pos)) in sorted.into_iter() {
        if distance(center - *pos) >= dist {
            continue;
        }

        if let Some(ChunkData::Generated(chunk)) = world.chunks.get(pos) {
            let blocks = BLOCKS.read().unwrap();

            // SAFETY: All NULL values are overwritten
            #[allow(invalid_value)]
            let mut neighbors: [Border; 6] = unsafe { zeroed() };
            for d in Direction::all() {
                match world.chunks.get(&(*pos + IVec3::from(d))) {
                    Some(ChunkData::Visible(chunk) | ChunkData::Generated(chunk)) => {
                        neighbors[d as usize] = chunk.border(d.inverse(), &blocks);
                    }
                    _ => {
                        info!("skip drawing {pos}");
                        continue 'entities;
                    }
                }
            }
            info!("drawing {pos}");

            // takes a lot of time!
            let mesh = chunk.mesh(neighbors);

            cmds.entity(entity)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(mesh),
                    material: block_mat.0.clone(),
                    transform: Transform::from_translation(VoxelWorld::world_pos(*pos)),
                    ..default()
                })
                .insert(VisibleChunk(*pos))
                .remove::<GeneratedChunk>();

            world
                .chunks
                .entry(*pos)
                .and_replace_entry_with(|_, v| match v {
                    ChunkData::Generated(v) => Some(ChunkData::Visible(v)),
                    _ => None,
                });

            return;
        }
    }
}

fn despawn_chunks(
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    settings: Res<PlayerSettings>,
    query: Query<&Transform, With<PlayerController>>,
    visible_query: Query<(Entity, &VisibleChunk)>,
    generated_query: Query<(Entity, &GeneratedChunk)>,
) {
    let player_transform = query.single();
    let center = VoxelWorld::chunk_pos(player_transform.translation);

    let dist = settings.view_distance as u32;

    visible_query.for_each(|(entity, VisibleChunk(pos))| {
        if distance(center - *pos) > dist {
            info!("despawn {pos}");
            cmds.entity(entity).despawn();
            world.chunks.remove(pos);
        }
    });

    generated_query.for_each(|(entity, GeneratedChunk(pos))| {
        if distance(center - *pos) > dist + 1 {
            info!("despawn {pos}");
            cmds.entity(entity).despawn();
            world.chunks.remove(pos);
        }
    });
}

fn distance(p: IVec3) -> u32 {
    p.max_element().abs().max(p.min_element().abs()) as _
}

pub struct RegenerateEvent;

fn regenerate_chunks(
    mut events: EventReader<RegenerateEvent>,
    mut cmds: Commands,
    mut world: ResMut<VoxelWorld>,
    chunk_query: Query<Entity, With<VisibleChunk>>,
    visible_query: Query<Entity, With<GeneratedChunk>>,
    generating_query: Query<Entity, With<ChunkResult>>,
) {
    let mut regenerate = false;
    for _ in events.iter() {
        regenerate = true;
    }
    if regenerate {
        warn!("Regenerate!");
        chunk_query.for_each(|entity| cmds.entity(entity).despawn());
        visible_query.for_each(|entity| cmds.entity(entity).despawn());
        generating_query.for_each(|entity| cmds.entity(entity).despawn());
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
            .add_system_set(
                SystemSet::on_update(AppState::Running)
                    .with_system(init_generation)
                    .with_system(handle_generation)
                    .with_system(mesh_generation)
                    .with_system(despawn_chunks)
                    .with_system(regenerate_chunks)
                    .with_system(move_chunk_center),
            );
    }
}
