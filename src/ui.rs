use std::time::Duration;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use bevy::time::common_conditions::on_timer;
use bevy_egui::egui::plot::{Line, Plot, PlotPoints};
use bevy_egui::egui::{RichText, Slider};
use bevy_egui::{egui, EguiContexts};

use crate::generation::WorldGen;
use crate::player::{PlayerController, PlayerSettings};
use crate::world::RegenerateEvent;
use crate::{AppState, BlockMat};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FPSHistory>()
            .add_systems(
                Update,
                fps_history.run_if(on_timer(Duration::from_secs_f32(0.2))),
            )
            .add_systems(Update, update.run_if(in_state(AppState::Running)));
    }
}

#[derive(Default, Resource)]
pub struct FPSHistory {
    values: [f64; Self::N],
    index: usize,
}

impl FPSHistory {
    const N: usize = 16;

    fn add(&mut self, value: f64) {
        self.values[self.index] = value;
        self.index = (self.index + 1) % Self::N;
    }

    fn get(&self) -> f64 {
        self.values[self.index.checked_sub(1).unwrap_or(Self::N - 1)]
    }

    /// History: oldest -> newest
    fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        [&self.values[self.index..], &self.values[..self.index]]
            .into_iter()
            .flatten()
            .copied()
    }
}

pub fn fps_history(mut fps_history: ResMut<FPSHistory>, diagnostics: Res<DiagnosticsStore>) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(avg) = fps.average() {
            fps_history.add(avg);
        }
    }
}

/// UI update function
pub fn update(
    mut egui_context: EguiContexts,
    fps: Res<FPSHistory>,
    mut player_settings: ResMut<PlayerSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut noise: ResMut<WorldGen>,
    block_mat: Res<BlockMat>,
    mut events: EventWriter<RegenerateEvent>,
    player_controller: Query<(&PlayerController, &Transform)>,
) {
    let (p_movement, p_transform) = player_controller.single();

    egui::Window::new("Settings").show(egui_context.ctx_mut(), |ui| {
        ui.label(format!("FPS: {:.3}", fps.get()));

        let measurements = fps
            .iter()
            .enumerate()
            .map(|(i, value)| [-(i as f64), value]);

        let line = Line::new(PlotPoints::from_iter(measurements));

        let max = fps.iter().reduce(f64::max).unwrap_or_default().max(60.0) + 30.0;

        Plot::new("fps")
            .view_aspect(4.0)
            .include_y(0.0)
            .include_y(max)
            .show(ui, |plot_ui| plot_ui.line(line));

        ui.label(RichText::new("Player Settings").heading());
        ui.add(Slider::new(&mut player_settings.m_speed, 0.0..=50.0).text("M Speed"));
        ui.add(Slider::new(&mut player_settings.m_acceleration, 0.0..=10.0).text("M Acceleration"));
        ui.add(Slider::new(&mut player_settings.m_deceleration, 0.0..=10.0).text("M Deceleration"));
        ui.add(Slider::new(&mut player_settings.r_speed, 0.0..=2.0).text("R Speed"));
        ui.add(Slider::new(&mut player_settings.view_distance, 1..=12).text("View Distance"));

        ui.separator();

        ui.label(RichText::new("Player Movement").heading());
        ui.label(format!("Yaw: {:.2}", p_movement.yaw));
        ui.label(format!("Pitch: {:.2}", p_movement.pitch));
        ui.label(format!("Time: {:.2}", p_movement.time));
        ui.label(format!("Velocity: {:.2?}", p_movement.velocity));
        ui.label(format!("Position: {:.2?}", p_transform.translation));
    });

    egui::Window::new("Block Material").show(egui_context.ctx_mut(), |ui| {
        if let Some(mat) = materials.get_mut(&block_mat.0) {
            ui.add(Slider::new(&mut mat.metallic, 0.0..=1.0).text("Metallic"));
            ui.add(Slider::new(&mut mat.perceptual_roughness, 0.0..=1.0).text("Roughness"));
            ui.add(Slider::new(&mut mat.reflectance, 0.0..=1.0).text("Reflectance"));
        }
    });

    egui::Window::new("World Generation").show(egui_context.ctx_mut(), |ui| {
        ui.label("Height");
        ui.add(Slider::new(&mut noise.height.start, -8.0 * 32.0..=8.0 * 32.0).text("min"));
        ui.add(Slider::new(&mut noise.height.end, -8.0 * 32.0..=8.0 * 32.0).text("max"));

        ui.separator();

        ui.label("3D Noise");
        ui.add(Slider::new(&mut noise.base.octaves, 1..=10).text("Octaves"));
        ui.add(Slider::new(&mut noise.base.frequency, 0.0..=10.0).text("Frequency"));
        ui.add(Slider::new(&mut noise.base.lacunarity, 0.0..=10.0).text("Lacunarity"));
        ui.add(Slider::new(&mut noise.base.persistence, 0.0..=10.0).text("Persistance"));
        ui.add(Slider::new(&mut noise.base.attenuation, 0.0..=10.0).text("Attenuation"));
        let max = noise.base.octaves as f32;
        ui.add(Slider::new(&mut noise.base_limit.start, -max..=max).text("Min"));
        ui.add(Slider::new(&mut noise.base_limit.end, -max..=max).text("Max"));
        ui.add(Slider::new(&mut noise.base_strength, 0.0..=1.0).text("Strength"));

        ui.separator();

        ui.label("Dirt Range");
        ui.add(Slider::new(&mut noise.dirt_range.start, -8 * 32..=8 * 32).text("min"));
        ui.add(Slider::new(&mut noise.dirt_range.end, -8 * 32..=8 * 32).text("max"));

        ui.add(Slider::new(&mut noise.dirt_height, 1..=10).text("Dirt"));

        ui.separator();

        if ui.button("Regenerate").clicked() {
            events.send(RegenerateEvent);
        }
    });
}
