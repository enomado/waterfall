use cpal::traits::{DeviceTrait, HostTrait};
use eframe::{egui_glow, glow};
use egui::{mutex::Mutex, ScrollArea};
use std::sync::Arc;

use crate::backend::{self};

pub mod debug_plot;
use debug_plot::DebugPlots;
mod waterfall;
use waterfall::Waterfall;
mod fft;
use fft::Fft;
pub mod turbo_colormap;

const FFT_SIZE: usize = 1024;

pub struct TemplateApp {}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let host = cpal::default_host();

        let device = host.default_input_device().unwrap();

        let config = device.default_input_config().unwrap();

        dbg!(config.sample_format());

        let config = config.config();

        let stream = device
            .build_input_stream(
                &config,
                |s: &[f32], t| {
                    dbg!(s);
                },
                |s| {
                    dbg!(s);
                },
                Some(core::time::Duration::from_secs(5)),
            )
            .unwrap();

        Self {}
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    /// Currently does nothing
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called once on shutdown, after [`Self::save`].
    fn on_exit(&mut self, gl: Option<&glow::Context>) {}

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        // Menu bar panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {});
        });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::top("Plot")
                .resizable(true)
                .show_inside(ui, |_ui| {
                    // TODO: Add plot
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // powered_by_egui_and_eframe(ui);
                    egui::warn_if_debug_build(ui);
                });
            });
        });
    }
}
