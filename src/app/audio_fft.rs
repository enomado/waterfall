use anyhow::{anyhow, Result};
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
    BufferSize, StreamConfig,
};
use realfft::RealFftPlanner;
use std::sync::mpsc::{self, Sender};

use super::debug_plot::PlotData;

pub struct AudioFFT {
    pub stream: cpal::Stream,
    pub output_len: usize,
}

impl AudioFFT {
    pub fn new(
        size: usize,
        plot_tx: Sender<(&'static str, PlotData)>,
    ) -> Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let output_len = size / 2 + 1;

        // Create mpsc queue
        let (tx, rx) = mpsc::channel();

        // Setup fft use f32 for now
        let mut fft_planner = RealFftPlanner::<f32>::new();
        let fft = fft_planner.plan_fft_forward(size);

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

        let mut fft_in: Vec<f32> = Vec::with_capacity(size);
        let mut fft_out = fft.make_output_vec();
        let mut fft_scratch = fft.make_scratch_vec();
        let stream = device.build_input_stream(
            &config,
            move |mut data: &[f32], _: &cpal::InputCallbackInfo| {
                while data.fill_vec(&mut fft_in, size).is_ok() {
                    assert_eq!(size, fft_in.len());
                    fft.process_with_scratch(&mut fft_in, &mut fft_out, &mut fft_scratch)
                        .unwrap();
                    plot_tx
                        .send(("FFT Output", PlotData::Bode32(fft_out.clone())))
                        .unwrap();
                    fft_in.clear();
                    let output: Vec<u8> = fft_out.iter().map(|c| (c.arg() * 255.0) as u8).collect();
                    assert_eq!(output_len, output.len());
                    plot_tx
                        .send(("FFT Processed Output", PlotData::U8(output.clone())))
                        .unwrap();
                    tx.send(output).unwrap();
                }
            },
            move |err| log::error!("Audio Thread Error: {err}"),
            None,
        )?;

        Ok((Self { stream, output_len }, rx))
    }
}

trait FillVec {
    /// Takes elements from self and inserts them into out_vec
    /// Returns Ok if out_vec is filled to size
    /// Returns Err when out_vec is not fully filled (self will be empty)
    fn fill_vec(&mut self, out_vec: &mut Vec<f32>, size: usize) -> Result<()>;
}
impl FillVec for &[f32] {
    fn fill_vec(&mut self, out_vec: &mut Vec<f32>, size: usize) -> Result<()> {
        let have = self.len();
        if have == 0 {
            anyhow::bail!("Self empty");
        }
        let need = size - out_vec.len();
        let can_move = need.min(have);
        out_vec.extend_from_slice(&self[..can_move]);
        *self = &self[can_move..];
        match out_vec.len() == size {
            true => Ok(()),
            false => Err(anyhow!("out_vec not full")),
        }
    }
}
