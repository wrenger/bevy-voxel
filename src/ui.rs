use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use bevy_egui::egui::plot::{Line, Plot, PlotPoints};
use bevy_egui::egui::{RichText, Slider};
use bevy_egui::{egui, EguiContext};

use crate::generation::WorldGen;
use crate::player::{PlayerController, PlayerSettings};
use crate::world::RegenerateEvent;
use crate::BlockMat;

/// UI update function
pub fn update(
    mut egui_context: ResMut<EguiContext>,
    diagnostics: Res<Diagnostics>,
    time: Res<Time>,
    mut player_settings: ResMut<PlayerSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut noise: ResMut<WorldGen>,
    block_mat: Res<BlockMat>,
    mut events: EventWriter<RegenerateEvent>,
    player_controller: Query<(&PlayerController, &Transform)>,
) {
    let (p_movement, p_transform) = player_controller.single();

    egui::Window::new("Settings").show(egui_context.ctx_mut(), |ui| {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(avg) = fps.average() {
                ui.label(format!("FPS: {avg:.3}"));
            }

            let measurements = fps.measurements().map(|d| {
                [
                    -(time.last_update().unwrap_or(time.startup()) - d.time).as_secs_f64(),
                    d.value,
                ]
            });

            let line = Line::new(PlotPoints::from_iter(measurements));

            Plot::new("fps")
                .view_aspect(4.0)
                .include_y(0.0)
                .include_y(60.0)
                .show(ui, |plot_ui| plot_ui.line(line));
        }

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

        ui.label("Limits");
        let max = noise.octaves as f32;
        ui.add(Slider::new(&mut noise.limits.start, 1.0..=max).text("min"));
        ui.add(Slider::new(&mut noise.limits.end, 1.0..=max).text("max"));

        ui.separator();

        ui.add(Slider::new(&mut noise.freq, 0.0..=1.0).text("Freq"));
        ui.add(Slider::new(&mut noise.lacunarity, 0.0..=1.0).text("Lacunarity"));
        ui.add(Slider::new(&mut noise.gain, 0.0..=10.0).text("Gain"));
        ui.add(Slider::new(&mut noise.octaves, 1..=10).text("Octaves"));

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
