use bevy::asset::LoadState;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

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
use generation::Noise;
use player::PlayerMovementPlugin;
use textures::TextureMap;
use world::WorldPlugin;

use crate::block::BLOCKS;

fn main() {
    App::new()
        .init_resource::<ImageLoading>()
        .init_resource::<BlockLoading>()
        .init_resource::<BlockMat>()
        .init_resource::<Noise>()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(EguiPlugin)
        .add_asset::<BlockId>()
        .init_asset_loader::<BlockLoader>()
        .add_state(AppState::LoadTextures)
        .add_plugin(PlayerMovementPlugin)
        .add_plugin(WorldPlugin)
        .add_system_set(SystemSet::on_enter(AppState::LoadTextures).with_system(load_textures))
        .add_system_set(SystemSet::on_update(AppState::LoadTextures).with_system(check_textures))
        .add_system_set(SystemSet::on_exit(AppState::LoadTextures).with_system(build_textures))
        .add_system_set(SystemSet::on_enter(AppState::LoadBlocks).with_system(load_blocks))
        .add_system_set(SystemSet::on_update(AppState::LoadBlocks).with_system(check_blocks))
        .add_system_set(SystemSet::on_enter(AppState::Running).with_system(setup))
        .add_system_set(SystemSet::on_update(AppState::Running).with_system(ui::update))
        .run();
}

/// The different asset loading states of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppState {
    LoadTextures,
    LoadBlocks,
    Running,
}

#[derive(Default)]
struct ImageLoading(Vec<HandleUntyped>);

/// Load all block textures
fn load_textures(mut loading: ResMut<ImageLoading>, asset_server: Res<AssetServer>) {
    loading.0 = asset_server.load_folder("textures").unwrap();
}

/// Wait for the block texture loading
fn check_textures(
    mut state: ResMut<State<AppState>>,
    loading: Res<ImageLoading>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_group_load_state(loading.0.iter().map(|h| h.id)) {
        state.set(AppState::LoadBlocks).unwrap()
    }
}

/// Create the combined block texture atlas
fn build_textures(
    mut images: ResMut<Assets<Image>>,
    loading: Res<ImageLoading>,
    asset_server: Res<AssetServer>,
) {
    TextureMap::build(
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

#[derive(Default)]
struct BlockLoading(Vec<HandleUntyped>);

/// Load the block meshes.
fn load_blocks(mut loading: ResMut<BlockLoading>, asset_server: Res<AssetServer>) {
    loading.0 = asset_server.load_folder("blocks").unwrap();
}

/// Wait for the block meshes.
fn check_blocks(
    mut state: ResMut<State<AppState>>,
    loading: Res<BlockLoading>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded = asset_server.get_group_load_state(loading.0.iter().map(|h| h.id)) {
        state.set(AppState::Running).unwrap()
    }
}

#[derive(Default)]
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
        base_color_texture: Some(TextureMap::get().image()),
        metallic: 0.05,
        perceptual_roughness: 1.0,
        reflectance: 0.1,
        ..Default::default()
    });
    cmds.insert_resource(BlockMat(block_mat.clone()));

    // Spawn all available blocks
    for (i, handle) in loading.0.iter().enumerate() {
        let block_id = block_ids.get(handle).unwrap();
        let blocks = BLOCKS.read().unwrap();
        cmds.spawn_bundle(PbrBundle {
            mesh: meshes.add(blocks[&block_id].mesh()),
            material: block_mat.clone(),
            transform: Transform::from_xyz(2.0 + 2.0 * i as f32, 0.0, 0.0),
            ..default()
        });
    }

    let cube_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.2 }));

    // -x
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(0.5, 0.2, 0.2).into()),
        transform: Transform::from_xyz(-1.0, 0.0, 0.0),
        ..default()
    });
    // +x
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(1.0, 0.2, 0.2).into()),
        transform: Transform::from_xyz(1.0, 0.0, 0.0),
        ..default()
    });
    // -y
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(0.2, 0.5, 0.2).into()),
        transform: Transform::from_xyz(0.0, -1.0, 0.0),
        ..default()
    });
    // +y
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(0.2, 1.0, 0.2).into()),
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        ..default()
    });
    // -z
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(0.2, 0.2, 0.5).into()),
        transform: Transform::from_xyz(0.0, 0.0, -1.0),
        ..default()
    });
    // +z
    cmds.spawn_bundle(PbrBundle {
        mesh: cube_mesh.clone(),
        material: materials.add(Color::rgb(0.2, 0.2, 1.0).into()),
        transform: Transform::from_xyz(0.0, 0.0, 1.0),
        ..default()
    });

    // Quad displaying the generated block texture atlas
    cmds.spawn_bundle(PbrBundle {
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
        brightness: 0.5,
    });
}
