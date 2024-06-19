use anyhow::Result;
use core::panic;
use std::{
    sync::mpsc::{self, RecvTimeoutError, SyncSender, TrySendError},
    time::{Duration, Instant},
    usize,
};

use crate::app::debug_plot::{DebugPlotSender, PlotData};

const LUT_LEN: usize = 4096;

pub struct DummyDevice {
    close: SyncSender<()>,
}
impl DummyDevice {
    pub fn new(
        sample_rate: usize,
        fft_input: SyncSender<Vec<f32>>,
        _plot_tx: DebugPlotSender,
    ) -> Result<Self> {
        let sin_lut: Vec<f32> = (0..LUT_LEN)
            .map(|i| ((i as f32 / LUT_LEN as f32) * std::f32::consts::TAU).sin())
            .collect();
        let (close, close_rx) = mpsc::sync_channel(0);
        let buffer_size: usize = 2048;
        let loop_interval = Duration::from_secs_f32((1. / sample_rate as f32) * buffer_size as f32);
        let freq = (sample_rate / 4) as f32;
        let phase_delta = sin_lut.len() as f32 * (freq / sample_rate as f32);
        std::thread::spawn(move || {
            let mut phase = 0_f32;
            loop {
                let start = Instant::now();
                let samples: Vec<f32> = (0..buffer_size)
                    .map(|_i| {
                        phase = (phase + phase_delta) % sin_lut.len() as f32;
                        sin_lut[phase as usize]
                    })
                    .collect();
                _plot_tx
                    .send("Dummy output", PlotData::F32(samples.clone()))
                    .unwrap();
                match fft_input.try_send(samples) {
                    Ok(_) => {}
                    Err(TrySendError::Full(_)) => log::warn!("Dummy Backend buffer full."),
                    Err(TrySendError::Disconnected(_)) => {
                        panic!("Dummy device lost connection to frontend!")
                    }
                }
                match close_rx.recv_timeout(loop_interval - start.elapsed()) {
                    Ok(_) => break,
                    Err(RecvTimeoutError::Disconnected) => {
                        panic!("Dummy device lost connection to frontend!")
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        });

        Ok(Self { close })
    }
}
impl crate::backend::Device for DummyDevice {
    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO");
    }

    fn can_tune(&self) -> bool {
        false
    }

    fn tune(&mut self, _freq: usize) -> anyhow::Result<()> {
        anyhow::bail!("Can't tune this device")
    }

    fn close(self: Box<Self>) {
        self.close.send(()).unwrap();
    }
}

pub struct DummyBackend {
    sample_rate: usize,
}
impl DummyBackend {
    pub fn new() -> Self {
        Self { sample_rate: 48000 }
    }
}
impl super::Backend for DummyBackend {
    fn display_text(&self) -> &'static str {
        "Dummy"
    }

    fn show_device_selection(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO");
    }

    fn build_device(
        &mut self,
        fft_input: SyncSender<Vec<f32>>,
        _plot_tx: DebugPlotSender,
    ) -> anyhow::Result<Box<dyn super::Device>> {
        Ok(Box::new(DummyDevice::new(
            self.sample_rate,
            fft_input,
            _plot_tx,
        )?))
    }
}
