use std::sync::mpsc::Sender;

use egui::Ui;

use crate::app::debug_plot::PlotData;
mod audio;
pub trait Device {
    fn show_settings(&mut self, ui: &mut Ui);
    fn can_tune(&self) -> bool;
    fn tune(&mut self, freq: usize) -> anyhow::Result<()>;
}
pub trait Backend {
    fn display_text(&self) -> &'static str;
    fn show_device_selection(&mut self, ui: &mut Ui);
    fn build_device(
        &mut self,
        fft_input: Sender<Vec<f32>>,
        _plot_tx: Sender<(&'static str, PlotData)>,
    ) -> anyhow::Result<Box<dyn Device>>;
}
pub struct Backends(pub Vec<Box<dyn Backend>>);

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![Box::new(audio::AudioBackend::new())])
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![])
    }
}

#[cfg(target_os = "android")]
impl Default for Backends {
    fn default() -> Self {
        Backends(vec![])
    }
}
