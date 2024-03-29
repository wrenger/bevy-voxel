use bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::{asset::LoadState, pbr::DirectionalLightShadowMap};

mod block;
mod chunk;
mod generation;
mod player;
mod textures;
mod ui;
mod util;
mod world;

use bevy_egui::EguiPlugin;
use block::{BlockId, BlockLoader};
use chunk::Chunk;
use generation::WorldGen;
use player::PlayerMovementPlugin;
use textures::TileTextures;
use ui::UIPlugin;
use world::{ChunkCenter, WorldPlugin};

use crate::block::blocks;

fn main() {
    App::new()
        .init_resource::<ImageLoading>()
        .init_resource::<BlockLoading>()
        .init_resource::<BlockMat>()
        .init_resource::<WorldGen>()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins((DefaultPlugins, TemporalAntiAliasPlugin))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(EguiPlugin)
        .add_asset::<BlockId>()
        .init_asset_loader::<BlockLoader>()
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::LoadTextures), load_textures)
        .add_systems(
            Update,
            check_textures.run_if(in_state(AppState::LoadTextures)),
        )
        .add_systems(OnExit(AppState::LoadTextures), build_textures)
        .add_systems(OnEnter(AppState::LoadBlocks), load_blocks)
        .add_systems(Update, check_blocks.run_if(in_state(AppState::LoadBlocks)))
        .add_systems(OnEnter(AppState::Running), setup)
        // .add_systems(OnEnter(AppState::Running), debug_gizmos)
        .add_plugins(PlayerMovementPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(UIPlugin)
        .run();
}

/// The different asset loading states of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum AppState {
    #[default]
    LoadTextures,
    LoadBlocks,
    Running,
}

#[derive(Default, Resource)]
struct ImageLoading(Vec<HandleUntyped>);

/// Load all block textures
fn load_textures(mut loading: ResMut<ImageLoading>, asset_server: Res<AssetServer>) {
    loading.0 = asset_server.load_folder("textures").unwrap();
}

/// Wait for the block texture loading
fn check_textures(
    mut state: ResMut<NextState<AppState>>,
    loading: Res<ImageLoading>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_group_load_state(loading.0.iter().map(|h| h.id())) {
        state.set(AppState::LoadBlocks)
    }
}

/// Create the combined block texture atlas
fn build_textures(
    mut images: ResMut<Assets<Image>>,
    loading: Res<ImageLoading>,
    asset_server: Res<AssetServer>,
) {
    TileTextures::build(
        &loading
            .0
            .iter()
            .map(|t| t.clone_weak().typed())
            .collect::<Vec<_>>(),
        &asset_server,
        &mut images,
    )
    .unwrap();
}

#[derive(Default, Resource)]
struct BlockLoading(Vec<HandleUntyped>);

/// Load the block meshes.
fn load_blocks(mut loading: ResMut<BlockLoading>, asset_server: Res<AssetServer>) {
    loading.0 = asset_server.load_folder("blocks").unwrap();
}

/// Wait for the block meshes.
fn check_blocks(
    mut state: ResMut<NextState<AppState>>,
    loading: Res<BlockLoading>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_group_load_state(loading.0.iter().map(|h| h.id())) {
        state.set(AppState::Running)
    }
}

#[derive(Default, Resource)]
pub struct BlockMat(Handle<StandardMaterial>);

fn setup(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loading: Res<BlockLoading>,
    block_ids: Res<Assets<BlockId>>,
) {
    // The combined block material
    let block_mat = materials.add(StandardMaterial {
        base_color_texture: Some(TileTextures::get().image()),
        metallic: 0.05,
        perceptual_roughness: 1.0,
        reflectance: 0.1,
        ..Default::default()
    });
    cmds.insert_resource(BlockMat(block_mat.clone()));

    // Spawn all available blocks
    for (i, handle) in loading.0.iter().enumerate() {
        let block_id = block_ids.get(&handle.typed_weak()).unwrap();
        let blocks = blocks().read().unwrap();
        cmds.spawn(PbrBundle {
            mesh: meshes.add(blocks[block_id].mesh()),
            material: block_mat.clone(),
            transform: Transform::from_xyz(2.0 + 2.0 * i as f32, 0.0, 0.0),
            ..default()
        });
    }

    // Quad displaying the generated block texture atlas
    cmds.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            size: Vec2::new(4.0, 4.0),
            flip: false,
        })),
        material: block_mat,
        transform: Transform::from_xyz(0.0, 1.5, -2.0),
        ..default()
    });

    cmds.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });
}

#[allow(unused)]
fn debug_gizmos(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));

    cmds.spawn((ChunkCenter, SpatialBundle::default()))
        .with_children(|cmds| {
            let half = Chunk::SIZE as f32 / 2.0;
            // -x
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(0.5, 0.2, 0.2).into()),
                transform: Transform::from_xyz(-half, 0.0, 0.0),
                ..default()
            });
            // +x
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(1.0, 0.2, 0.2).into()),
                transform: Transform::from_xyz(half, 0.0, 0.0),
                ..default()
            });
            // -y
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(0.2, 0.5, 0.2).into()),
                transform: Transform::from_xyz(0.0, -half, 0.0),
                ..default()
            });
            // +y
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(0.2, 1.0, 0.2).into()),
                transform: Transform::from_xyz(0.0, half, 0.0),
                ..default()
            });
            // -z
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(0.2, 0.2, 0.5).into()),
                transform: Transform::from_xyz(0.0, 0.0, -half),
                ..default()
            });
            // +z
            cmds.spawn(PbrBundle {
                mesh: cube_mesh.clone(),
                material: materials.add(Color::rgb(0.2, 0.2, 1.0).into()),
                transform: Transform::from_xyz(0.0, 0.0, half),
                ..default()
            });
        });
}
