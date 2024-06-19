use std::sync::mpsc::SyncSender;

use egui::Ui;

use crate::app::debug_plot::DebugPlotSender;
mod audio;
mod dummy;
pub trait Device {
    fn show_settings(&mut self, ui: &mut Ui);
    fn can_tune(&self) -> bool;
    fn tune(&mut self, freq: usize) -> anyhow::Result<()>;
    fn close(self: Box<Self>);
}
pub trait Backend {
    fn display_text(&self) -> &'static str;
    fn show_device_selection(&mut self, ui: &mut Ui);
    fn build_device(
        &mut self,
        fft_input: SyncSender<Vec<f32>>,
        _plot_tx: DebugPlotSender,
    ) -> anyhow::Result<Box<dyn Device>>;
}
pub struct Backends(pub Vec<Box<dyn Backend>>);

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![
            Box::new(audio::AudioBackend::new()),
            Box::new(dummy::DummyBackend::new()),
        ])
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![Box::new(dummy::DummyBackend::new())])
    }
}

#[cfg(target_os = "android")]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![Box::new(dummy::DummyBackend::new())])
    }
}
