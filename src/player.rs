use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::WindowMode;

use crate::AppState;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSettings>()
            .add_system_set(SystemSet::on_enter(AppState::Running).with_system(setup))
            .add_system_set(SystemSet::on_update(AppState::Running).with_system(windowing))
            .add_system_set(SystemSet::on_update(AppState::Running).with_system(player_movement))
            .add_system_set(SystemSet::on_update(AppState::Running).with_system(move_lights));
    }
}

#[derive(Default, Component)]
pub struct PlayerController {
    pub yaw: f32,
    pub pitch: f32,
}

pub struct PlayerSettings {
    pub view_distance: usize,
    pub m_speed: f32,
    pub r_speed: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            view_distance: 6,
            m_speed: 20.0,
            r_speed: 0.5,
        }
    }
}

#[derive(Default, Component)]
struct PlayerLight;

fn setup(mut cmds: Commands) {
    cmds.spawn_bundle(PerspectiveCameraBundle {
        perspective_projection: PerspectiveProjection {
            fov: PI / 2.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(0.0, 1.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    })
    .insert(PlayerController {
        yaw: 0.0,
        pitch: 0.0,
    });

    // directional 'sun' light
    const HALF_SIZE: f32 = 16.0;
    cmds.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            rotation: Quat::from_euler(EulerRot::YXZ, FRAC_PI_4, -FRAC_PI_4, 0.0),
            ..default()
        },
        ..default()
    })
    .insert(PlayerLight);

    cmds.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        point_light: PointLight {
            intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
            color: Color::ORANGE,
            shadows_enabled: false,
            radius: 8.0,
            ..default()
        },
        ..default()
    })
    .insert(PlayerLight);
}

fn player_movement(
    key: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut mouse_move: EventReader<MouseMotion>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    mut query: Query<(&mut Transform, &mut PlayerController)>,
) {
    let (mut transform, mut movement) = query.single_mut();

    if mouse.pressed(MouseButton::Right) {
        if let Some(rotation) = mouse_move.iter().map(|m| m.delta).reduce(|a, e| a + e) {
            let mut new_pitch =
                movement.pitch + rotation.y * time.delta_seconds() * settings.r_speed;
            let new_yaw = movement.yaw + rotation.x * time.delta_seconds() * settings.r_speed;

            new_pitch = new_pitch.clamp(-FRAC_PI_2, FRAC_PI_2);

            movement.pitch = new_pitch;
            movement.yaw = new_yaw;

            transform.rotation = Quat::from_axis_angle(-Vec3::Y, new_yaw)
                * Quat::from_axis_angle(-Vec3::X, new_pitch);
        }
    }

    let dir = Vec3::new(
        key.pressed(KeyCode::D) as i32 as f32 - key.pressed(KeyCode::A) as i32 as f32,
        key.pressed(KeyCode::Space) as i32 as f32 - key.pressed(KeyCode::LShift) as i32 as f32,
        key.pressed(KeyCode::S) as i32 as f32 - key.pressed(KeyCode::W) as i32 as f32,
    )
    .clamp_length_max(1.0);

    if dir.length_squared() > f32::EPSILON {
        let velocity = Quat::from_axis_angle(-Vec3::Y, movement.yaw)
            * dir
            * time.delta_seconds()
            * settings.m_speed;
        transform.translation += velocity;
    }
}

// FIXME: The directional light shadows are not updated!
// Maybe only update directional light pos when entering new chunk?
fn move_lights(
    player: Query<&Transform, (With<PlayerController>, Changed<GlobalTransform>)>,
    mut lights: Query<&mut Transform, (With<PlayerLight>, Without<PlayerController>)>,
) {
    if let Ok(target) = player.get_single() {
        for mut transform in lights.iter_mut() {
            transform.translation = target.translation;
        }
    }
}

fn windowing(
    key: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut windows: ResMut<Windows>,
) {
    let window = windows.get_primary_mut().unwrap();
    if mouse.just_pressed(MouseButton::Right) {
        window.set_cursor_visibility(false);
        window.set_cursor_lock_mode(true);
    }
    if mouse.just_released(MouseButton::Right) {
        window.set_cursor_visibility(true);
        window.set_cursor_lock_mode(false);
    }

    if key.just_pressed(KeyCode::F11)
        || (key.pressed(KeyCode::LAlt) && key.just_pressed(KeyCode::Return))
    {
        window.set_mode(if window.mode() != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        });
    }
}
