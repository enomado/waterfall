use eframe::{egui_glow, glow};
use egui::{mutex::Mutex, ScrollArea};
use std::sync::Arc;

use crate::backend::{self, Backends};

pub mod debug_plot;
use debug_plot::DebugPlots;
mod waterfall;
use waterfall::Waterfall;
mod fft;
use fft::Fft;
pub mod turbo_colormap;

const FFT_SIZE: usize = 1024;

pub struct TemplateApp {
    plots: DebugPlots,
    // Example stuff:
    label: String,
    value: f32,
    /// Behind an `Arc<Mutex<…>>` so we can pass it to [`egui::PaintCallback`] and paint later.
    waterfall: Arc<Mutex<Waterfall>>,
    _fft: Fft,
    _backends: backend::Backends,
    _selected_backend: usize,
    _open_device: Option<Box<dyn backend::Device>>,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.

        let plots = DebugPlots::new();

        let (fft, rx) = Fft::new(FFT_SIZE, plots.get_sender()).unwrap();

        let wf_size = fft.output_len;
        let gl = cc
            .gl
            .as_ref()
            .expect("Could not get gl context from glow backend");

        Self {
            plots,
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            waterfall: Arc::new(Mutex::new(Waterfall::new(gl, wf_size, wf_size, rx))),
            //_stream: stream,
            _fft: fft,
            _backends: Backends::default(),
            _selected_backend: 0,
            _open_device: None,
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    /// Currently does nothing
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    /// Called once on shutdown, after [`Self::save`].
    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.waterfall.lock().destroy(gl);
        }
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        ctx.request_repaint();
        self.plots.update_plots();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
                self.plots.render_menu_buttons(ui);
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        self.plots.render_plot_windows(ctx);

        egui::Window::new("Select Device")
            .default_width(600.0)
            .default_height(400.0)
            .vscroll(false)
            .resizable(true)
            .show(ctx, |ui| {
                egui::SidePanel::left("Select Driver")
                    .resizable(true)
                    .default_width(150.0)
                    .width_range(80.0..=200.0)
                    .show_inside(ui, |ui| {
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    for (i, b) in self._backends.0.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut self._selected_backend,
                                            i,
                                            b.display_text(),
                                        );
                                    }
                                },
                            );
                        });
                    });
                //egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        //if self._selected_backend < self._backends.0.len() {
                        if let Some(b) = self._backends.0.get_mut(self._selected_backend) {
                            //let mut b = &self._backends.0[self._selected_backend];
                            b.show_device_selection(ui);
                            if ui.add(egui::Button::new("Apply")).clicked() {
                                drop(self._open_device.take());
                                if let Ok(device) =
                                    b.build_device(self._fft.tx.clone(), self.plots.get_sender())
                                {
                                    self._open_device = Some(device);
                                }
                            }
                        } else {
                            ui.add(egui::Label::new("Select a Device Driver"));
                        }
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("The texture is being painted using ");
                ui.hyperlink_to("glow", "https://github.com/grovesNL/glow");
                ui.label(" (OpenGL).");
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    let available_space = ui.available_size();
                    let (rect, response) =
                        ui.allocate_exact_size(available_space, egui::Sense::drag());

                    let _angle = response.drag_motion().x * 0.01;

                    // Clone locals so we can move them into the paint callback:
                    let waterfall = self.waterfall.clone();

                    let callback = egui::PaintCallback {
                        rect,
                        callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
                            move |_info, painter| {
                                waterfall.lock().paint(painter.gl(), _angle);
                            },
                        )),
                    };
                    ui.painter().add(callback);
                });
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
