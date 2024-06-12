use std::collections::HashMap;
use std::sync::mpsc;

use egui::{Context, Ui};
use egui_plot::{Line, Plot, PlotBounds, PlotPoints};
use realfft::num_complex::Complex32;

pub enum PlotData {
    U8(Vec<u8>),
    //F32(Vec<f32>),
    Bode32(Vec<Complex32>),
}
#[derive(Clone)]
pub struct DebugPlotSender {
    tx: mpsc::SyncSender<(&'static str, PlotData)>,
}
impl DebugPlotSender {
    pub fn send(
        &self,
        plot_name: &'static str,
        plot_data: PlotData,
    ) -> Result<(), mpsc::SendError<PlotData>> {
        match self.tx.try_send((plot_name, plot_data)) {
            Err(mpsc::TrySendError::Full(_)) => {
                log::warn!("Debug buffer is full!");
                Ok(())
            }
            Err(mpsc::TrySendError::Disconnected((_, d))) => Err(mpsc::SendError(d)),
            Ok(()) => Ok(()),
        }
    }
}
pub struct DebugPlots {
    plots: HashMap<&'static str, PlotData>,
    plot_en: HashMap<&'static str, bool>,
    rx: mpsc::Receiver<(&'static str, PlotData)>,
    tx: DebugPlotSender,
}

impl DebugPlots {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel(128);
        DebugPlots {
            plots: HashMap::new(),
            plot_en: HashMap::new(),
            rx,
            tx: DebugPlotSender { tx },
        }
    }
    pub fn get_sender(&self) -> DebugPlotSender {
        self.tx.clone()
    }
    pub fn update_plots(&mut self) {
        while let Ok((key, plot)) = self.rx.try_recv() {
            if self.plots.insert(key, plot).is_none() {
                self.plot_en.insert(key, false);
            }
        }
    }
    pub fn render_menu_buttons(&mut self, ui: &mut Ui) {
        ui.menu_button("Debug Plots", |ui| {
            for &k in self.plots.keys() {
                if !self.plot_en.contains_key(k) {
                    self.plot_en.insert(k, false);
                }
                let enabled = self.plot_en.get_mut(k).unwrap();
                ui.checkbox(enabled, k);
            }
        });
    }
    pub fn render_plot_windows(&mut self, ctx: &Context) {
        for (key, plot) in self.plots.iter() {
            let enabled = self.plot_en.get_mut(key).unwrap();
            if *enabled {
                DebugPlots::render_window(ctx, key, plot, enabled);
            }
        }
    }
    fn render_window(ctx: &Context, title: &'static str, plot: &PlotData, open: &mut bool) {
        egui::Window::new(title).open(open).show(ctx, |ui| {
            ui.heading(title);
            match plot {
                PlotData::U8(v) => {
                    ui.heading("u8 Plot");
                    let line = Line::new(PlotPoints::from_iter(
                        v.iter().enumerate().map(|(i, y)| [i as f64, *y as f64]),
                    ));
                    let plot = Plot::new(title);
                    plot.show(ui, |plot_ui| {
                        plot_ui.line(line);
                        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                            [-1.0, -1.0],
                            [(v.len() + 1) as f64, core::u8::MAX as f64 + 1.0],
                        ));
                    });
                }
                PlotData::Bode32(v) => {
                    ui.heading("Bode Plot");
                    let mag_line =
                        Line::new(PlotPoints::from_iter(v.iter().enumerate().map(|(i, c)| {
                            [
                                i as f64,
                                ((c.re * c.re) + (c.im * c.im)).sqrt() as f64 / v.len() as f64,
                            ]
                        })));
                    let phase_line = Line::new(PlotPoints::from_iter(
                        v.iter()
                            .enumerate()
                            .map(|(i, c)| [i as f64, c.arg() as f64 / core::f64::consts::PI]),
                    ));
                    let plot = Plot::new(title);
                    plot.show(ui, |plot_ui| {
                        plot_ui.line(mag_line);
                        plot_ui.line(phase_line);
                        plot_ui.set_plot_bounds(PlotBounds::from_min_max(
                            [0.0, -1.0],
                            [(v.len() + 1) as f64, 1.0],
                        ));
                    });
                }
            };
        });
    }
}
