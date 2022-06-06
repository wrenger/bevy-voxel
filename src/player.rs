use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::AppState;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSettings>()
            .add_system_set(SystemSet::on_update(AppState::Running).with_system(player_movement));
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
            view_distance: 4,
            m_speed: 10.0,
            r_speed: 0.5,
        }
    }
}

fn player_movement(
    key: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut mouse_move: EventReader<MouseMotion>,
    time: Res<Time>,
    settings: Res<PlayerSettings>,
    mut windows: ResMut<Windows>,
    mut query: Query<(&mut Transform, &mut PlayerController)>,
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

    let rotation = mouse_move
        .iter()
        .map(|m| m.delta)
        .reduce(|a, e| a + e)
        .unwrap_or_default();

    for (mut transform, mut movement) in query.iter_mut() {
        if mouse.pressed(MouseButton::Right) {
            let mut new_pitch =
                movement.pitch + rotation.y * time.delta_seconds() * settings.r_speed;
            let new_yaw = movement.yaw + rotation.x * time.delta_seconds() * settings.r_speed;

            new_pitch = new_pitch.clamp(-FRAC_PI_2, FRAC_PI_2);

            movement.pitch = new_pitch;
            movement.yaw = new_yaw;

            transform.rotation = Quat::from_axis_angle(-Vec3::Y, new_yaw)
                * Quat::from_axis_angle(-Vec3::X, new_pitch);
        }

        let dir = Vec3::new(
            key.pressed(KeyCode::D) as i32 as f32 - key.pressed(KeyCode::A) as i32 as f32,
            key.pressed(KeyCode::Space) as i32 as f32 - key.pressed(KeyCode::LShift) as i32 as f32,
            key.pressed(KeyCode::S) as i32 as f32 - key.pressed(KeyCode::W) as i32 as f32,
        )
        .clamp_length_max(1.0);

        let velocity = Quat::from_axis_angle(-Vec3::Y, movement.yaw)
            * dir
            * time.delta_seconds()
            * settings.m_speed;
        transform.translation += velocity;
    }
}
