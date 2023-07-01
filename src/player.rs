use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use bevy::core_pipeline::fxaa::Fxaa;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::camera::Projection;
use bevy::window::{CursorGrabMode, PrimaryWindow, WindowMode};

use crate::chunk::Chunk;
use crate::util::RangeExt;
use crate::AppState;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSettings>()
            .add_system(setup.in_schedule(OnEnter(AppState::Running)))
            .add_systems(
                (windowing, player_movement, move_lights)
                    .chain()
                    .in_set(OnUpdate(AppState::Running)),
            );
    }
}

#[derive(Default, Component)]
pub struct PlayerController {
    pub yaw: f32,
    pub pitch: f32,
    pub time: f32,
    pub velocity: Vec3,
}

#[derive(Resource)]
pub struct PlayerSettings {
    pub view_distance: usize,
    pub m_speed: f32,
    pub m_acceleration: f32,
    pub m_deceleration: f32,
    pub r_speed: f32,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            view_distance: 6,
            m_speed: 35.0,
            m_acceleration: 4.0,
            m_deceleration: 10.0,
            r_speed: 0.5,
        }
    }
}

/// The player light that should be moved with the player.
/// The parameter configures if the position should be rounded.
#[derive(Default, Component)]
struct PlayerLight;

/// Create the player
fn setup(mut cmds: Commands) {
    cmds.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: PI / 2.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PlayerController::default(),
        Fxaa::default()
    ));

    // directional 'sun' light
    const HALF_SIZE: f32 = Chunk::SIZE as f32 * 8.0;
    cmds.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 8.0,
            maximum_distance: HALF_SIZE,
            overlap_proportion: 0.4,
            ..default()
        }
        .into(),
        transform: Transform {
            rotation: Quat::from_euler(EulerRot::YXZ, FRAC_PI_4, -FRAC_PI_4, 0.0),
            ..default()
        },
        ..default()
    });

    cmds.spawn((
        PlayerLight,
        PointLightBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::ORANGE,
                shadows_enabled: false,
                radius: 8.0,
                ..default()
            },
            ..default()
        },
    ));
}

/// Handle player movement and rotation
fn player_movement(
    key: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut mouse_move: EventReader<MouseMotion>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    mut query: Query<(&mut Transform, &mut PlayerController)>,
) {
    let (mut transform, mut movement) = query.single_mut();

    // Rotate the player via the mouse move event
    if mouse.pressed(MouseButton::Right) {
        if let Some(rotation) = mouse_move.iter().map(|m| m.delta).reduce(|a, e| a + e) {
            let new_pitch = (movement.pitch + rotation.y * time.delta_seconds() * settings.r_speed)
                .clamp(-FRAC_PI_2, FRAC_PI_2);
            let new_yaw =
                (movement.yaw + rotation.x * time.delta_seconds() * settings.r_speed) % TAU;

            movement.pitch = new_pitch;
            movement.yaw = new_yaw;

            transform.rotation = Quat::from_axis_angle(-Vec3::Y, new_yaw)
                * Quat::from_axis_angle(-Vec3::X, new_pitch);
        }
    }

    // Get the movement direction from the user input
    let dir = Vec3::new(
        key.pressed(KeyCode::D) as i32 as f32 - key.pressed(KeyCode::A) as i32 as f32,
        key.pressed(KeyCode::Space) as i32 as f32 - key.pressed(KeyCode::LShift) as i32 as f32,
        key.pressed(KeyCode::S) as i32 as f32 - key.pressed(KeyCode::W) as i32 as f32,
    )
    .clamp_length_max(1.0);

    let actively_moving = dir.length_squared() > f32::EPSILON;

    // Apply different accelerations based depending if the player accelerates or decelerates
    let boost = if actively_moving {
        // Activate deceleration boost after reaching 80% of the max possible movement speed
        if movement.time < 0.8 {
            movement.time =
                (movement.time..1.0).lerp(time.delta_seconds() * settings.m_acceleration);
            settings.m_acceleration
        } else {
            settings.m_deceleration
        }
    } else {
        movement.time = 0.0;
        settings.m_deceleration
    };

    // Update the new player position
    if actively_moving || movement.velocity.length_squared() > f32::EPSILON {
        let velocity = movement.velocity.lerp(
            Quat::from_axis_angle(-Vec3::Y, movement.yaw) * dir * settings.m_speed,
            time.delta_seconds() * boost,
        );
        transform.translation += velocity * time.delta_seconds();
        movement.velocity = velocity;
    }
}

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

/// Update the window on mouse lock / fullscreen
fn windowing(
    key: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = windows.single_mut();

    if mouse.just_pressed(MouseButton::Right) {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
    if mouse.just_released(MouseButton::Right) {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
    }

    if key.just_pressed(KeyCode::F11)
        || (key.pressed(KeyCode::LAlt) && key.just_pressed(KeyCode::Return))
    {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
}
