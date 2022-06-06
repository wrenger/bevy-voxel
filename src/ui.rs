use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use bevy_egui::{
    egui::{
        self,
        plot::{Line, Plot, Value, Values},
        Slider,
    },
    EguiContext,
};

use crate::player::PlayerSettings;

#[derive(Default)]
pub struct BlockMat {
    pub mat: Option<Handle<StandardMaterial>>,
}

pub fn update(
    mut egui_context: ResMut<EguiContext>,
    diagnostics: Res<Diagnostics>,
    time: Res<Time>,
    mut player_settings: ResMut<PlayerSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    block_mat: Res<BlockMat>,
) {
    egui::Window::new("Settings").show(egui_context.ctx_mut(), |ui| {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(avg) = fps.average() {
                ui.label(format!("FPS: {avg:.3}"));
            }

            let line = Line::new(Values::from_values_iter(fps.measurements().map(|d| {
                Value::new(
                    -(time.last_update().unwrap_or(time.startup()) - d.time).as_secs_f32(),
                    d.value,
                )
            })));

            Plot::new("fps")
                .view_aspect(4.0)
                .include_y(0.0)
                .include_y(60.0)
                .show(ui, |plot_ui| plot_ui.line(line));
        }

        ui.label("Movement Speed:");
        ui.add(Slider::new(&mut player_settings.m_speed, 0.0..=50.0));

        ui.label("Rotation Speed:");
        ui.add(Slider::new(&mut player_settings.r_speed, 0.0..=2.0));
    });

    egui::Window::new("Block Material").show(egui_context.ctx_mut(), |ui| {
        if let Some(mat) = block_mat.mat.as_ref().and_then(|m| materials.get_mut(m)) {
            ui.label("Metallic");
            ui.add(Slider::new(&mut mat.metallic, 0.0..=1.0));
            ui.label("Roughness");
            ui.add(Slider::new(&mut mat.perceptual_roughness, 0.0..=1.0));
            ui.label("Reflectance");
            ui.add(Slider::new(&mut mat.reflectance, 0.0..=1.0));
        }
    });
}
