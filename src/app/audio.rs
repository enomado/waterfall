use anyhow::{anyhow, Result};
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
    BufferSize, StreamConfig,
};
use std::sync::mpsc::Sender;

use super::debug_plot::PlotData;

pub struct Audio {
    pub stream: cpal::Stream,
}

impl Audio {
    pub fn new(
        fft_input: Sender<Vec<f32>>,
        _plot_tx: Sender<(&'static str, PlotData)>,
    ) -> Result<Self> {
        // Setup audio input
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(anyhow!("No input audio device found"))?;
        // Basic config that 'should' be suppoted by most devices
        let config = StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(44100),
            buffer_size: BufferSize::Default,
        };

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                fft_input.send(data.to_vec()).unwrap();
            },
            move |err| log::error!("Audio Thread Error: {err}"),
            None,
        )?;

        Ok(Self { stream })
    }
}
