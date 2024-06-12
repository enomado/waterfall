use anyhow::Result;
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
    BufferSize,
};
use std::sync::mpsc::{SyncSender, TrySendError};

use crate::app::debug_plot::DebugPlotSender;

pub struct Audio {
    pub stream: cpal::Stream,
}
impl Audio {
    pub fn new(
        device: &cpal::Device,
        config: cpal::StreamConfig,
        fft_input: SyncSender<Vec<f32>>,
        _plot_tx: DebugPlotSender,
    ) -> Result<Self> {
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                match fft_input.try_send(data.to_vec()) {
                    Err(TrySendError::Disconnected(_)) => panic!(
                        "Error: Audio backend has lost connection to frontend! Can not continue!"
                    ),
                    Err(TrySendError::Full(_)) => log::warn!("Audio Backend buffer full."),
                    Ok(()) => {}
                };
            },
            move |err| log::error!("Audio Thread Error: {err}"),
            None,
        )?;

        Ok(Self { stream })
    }
}
impl crate::backend::Device for Audio {
    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO");
    }

    fn can_tune(&self) -> bool {
        false
    }

    fn tune(&mut self, _freq: usize) -> anyhow::Result<()> {
        anyhow::bail!("Can't tune this device")
    }
}

pub struct AudioBackend {
    host: cpal::Host,
    devices: Vec<cpal::Device>,
    current_device: usize,
}
impl AudioBackend {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let devices = host.devices().unwrap().collect();
        let current_device = 0;
        Self {
            host,
            devices,
            current_device,
        }
    }
    fn update_devices(&mut self) {
        self.devices.clear();
        self.devices = self.host.devices().unwrap().collect();
        self.current_device = 0;
    }
}
impl super::Backend for AudioBackend {
    fn display_text(&self) -> &'static str {
        "Audio"
    }

    fn show_device_selection(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_label("Device")
            .selected_text(
                self.devices[self.current_device]
                    .name()
                    .unwrap_or("UNKNOWN DEVICE".into()),
            )
            .show_index(ui, &mut self.current_device, self.devices.len(), |i| {
                self.devices[i].name().unwrap_or("UNKNOWN DEVICE".into())
            });
        if ui.add(egui::Button::new("Refresh")).clicked() {
            self.update_devices();
        }
    }

    fn build_device(
        &mut self,
        fft_input: SyncSender<Vec<f32>>,
        _plot_tx: DebugPlotSender,
    ) -> anyhow::Result<Box<dyn super::Device>> {
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(44100),
            buffer_size: BufferSize::Default,
        };
        Ok(Box::new(Audio::new(
            &self.devices[self.current_device],
            config,
            fft_input,
            _plot_tx,
        )?))
    }
}