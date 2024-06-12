use anyhow::{anyhow, Result};
use realfft::RealFftPlanner;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

use super::debug_plot::{DebugPlotSender, PlotData};

pub struct Fft {
    pub tx: SyncSender<Vec<f32>>,
    pub output_len: usize,
}

impl Fft {
    pub fn new(size: usize, plot_tx: DebugPlotSender) -> Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let output_len = size / 2 + 1;

        // Create mpsc queue
        let (tx, rx) = mpsc::sync_channel(10);
        let (in_tx, in_rx): (SyncSender<Vec<f32>>, Receiver<Vec<f32>>) = mpsc::sync_channel(10);

        // Setup fft use f32 for now
        let mut fft_planner = RealFftPlanner::<f32>::new();
        let fft = fft_planner.plan_fft_forward(size);

        let mut fft_in: Vec<f32> = Vec::with_capacity(size);
        let mut fft_out = fft.make_output_vec();
        let mut fft_scratch = fft.make_scratch_vec();

        std::thread::spawn(move || {
            while let Ok(samples) = in_rx.recv() {
                let mut data = samples.as_slice();
                while data.fill_vec(&mut fft_in, size).is_ok() {
                    assert_eq!(size, fft_in.len());
                    fft.process_with_scratch(&mut fft_in, &mut fft_out, &mut fft_scratch)
                        .unwrap();
                    plot_tx
                        .send("FFT Output", PlotData::Bode32(fft_out.clone()))
                        .unwrap();
                    fft_in.clear();
                    let output: Vec<u8> = fft_out
                        .iter()
                        .map(|c| {
                            (((c.re * c.re) + (c.im * c.im)).sqrt() / output_len as f32 * 255.0)
                                as u8
                        })
                        .collect();
                    assert_eq!(output_len, output.len());
                    plot_tx
                        .send("FFT Processed Output", PlotData::U8(output.clone()))
                        .unwrap();
                    match tx.try_send(output) {
                        Ok(_) => {}
                        Err(TrySendError::Full(_)) => log::warn!("Waterfall buffer full."),
                        Err(TrySendError::Disconnected(_)) => {
                            panic!("The fft thread has disconnected from the waterfall!")
                        }
                    }
                }
            }
        });

        Ok((
            Self {
                tx: in_tx,
                output_len,
            },
            rx,
        ))
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
