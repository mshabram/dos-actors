use super::ProgressBar;
use crate::{
    io::{Data, UniqueIdentifier, Write},
    Update,
};
use linya::{Bar, Progress};
use std::{
    ops::Add,
    sync::{Arc, Mutex},
};

#[cfg(feature = "noise")]
use rand_distr::{Distribution, Normal, NormalError};

/// Signal types
#[derive(Debug, Clone)]
pub enum Signal {
    /// A constant signal
    Constant(f64),
    /// A sinusoidal signal
    Sinusoid {
        amplitude: f64,
        sampling_frequency_hz: f64,
        frequency_hz: f64,
        phase_s: f64,
    },
    /// A ramp of the for y=ax+b
    Ramp { a: f64, b: f64 },
    /// A sigmoid
    Sigmoid {
        amplitude: f64,
        sampling_frequency_hz: f64,
    },
    /// White noise
    #[cfg(feature = "noise")]
    WhiteNoise(Normal<f64>),
    /// A simphony?
    Composite(Vec<Signal>),
}

#[cfg(feature = "noise")]
impl Signal {
    /// Create a white noise signal with a standard deviation equal to one
    pub fn white_noise() -> Result<Self, NormalError> {
        Ok(Signal::WhiteNoise(Normal::new(0f64, 1f64)?))
    }
    /// Sets white noise standard deviation
    pub fn std_dev(self, sigma: f64) -> Result<Self, NormalError> {
        if let Signal::WhiteNoise(noise) = self {
            Ok(Signal::WhiteNoise(Normal::new(noise.mean(), sigma)?))
        } else {
            Ok(self)
        }
    }
    /// Adds bias to white noise
    pub fn bias(self, bias: f64) -> Result<Self, NormalError> {
        if let Signal::WhiteNoise(noise) = self {
            Ok(Signal::WhiteNoise(Normal::new(bias, noise.std_dev())?))
        } else {
            Ok(self)
        }
    }
}
impl Signal {
    /// Returns the signal value at step `i`
    pub fn get(&self, i: usize) -> f64 {
        use Signal::*;
        match self {
            Constant(val) => *val,
            Sinusoid {
                amplitude,
                sampling_frequency_hz,
                frequency_hz,
                phase_s,
            } => {
                (2f64
                    * std::f64::consts::PI
                    * (phase_s + i as f64 * frequency_hz / sampling_frequency_hz))
                    .sin()
                    * amplitude
            }
            Ramp { a, b } => a * i as f64 + b,
            Sigmoid {
                amplitude,
                sampling_frequency_hz,
            } => {
                let u = i as f64 / sampling_frequency_hz - 0.75;
                let r = (1. + (-5. * u).exp()).recip();
                amplitude * r * r
            }
            #[cfg(feature = "noise")]
            WhiteNoise(noise) => noise.sample(&mut rand::thread_rng()),
            Composite(signals) => signals.iter().map(|signal| signal.get(i)).sum(),
        }
    }
}

/// Signals generator
#[derive(Debug)]
pub struct Signals {
    size: usize,
    pub signals: Vec<Signal>,
    pub step: usize,
    pub n_step: usize,
    progress_bar: Option<ProgressBar>,
}
impl Signals {
    /// Create `n` null [Signal::Constant]s valid for `n_step` iterations
    pub fn new(n: usize, n_step: usize) -> Self {
        let signals: Vec<_> = vec![Signal::Constant(0f64); n];
        Self {
            size: n,
            signals,
            step: 0,
            n_step,
            progress_bar: None,
        }
    }
    pub fn progress(self) -> Self {
        let mut progress = Progress::new();
        let bar: Bar = progress.bar(self.n_step, "Signal(s):");
        Self {
            progress_bar: Some(ProgressBar {
                progress: Arc::new(Mutex::new(progress)),
                bar,
            }),
            ..self
        }
    }
    /// Sets the same [Signal] for all outputs
    pub fn signals(self, signal: Signal) -> Self {
        let signals = vec![signal.clone(); self.size];
        Self { signals, ..self }
    }
    /// Sets the [Signal] of output #`k`
    pub fn output_signal(self, k: usize, output_signal: Signal) -> Self {
        let mut signals = self.signals;
        signals[k] = output_signal;
        Self { signals, ..self }
    }
}

impl From<(Vec<f64>, usize)> for Signals {
    fn from((data, n_step): (Vec<f64>, usize)) -> Self {
        let n = data.len();
        data.into_iter()
            .enumerate()
            .fold(Signals::new(n, n_step), |s, (i, v)| {
                s.output_signal(i, Signal::Constant(v))
            })
    }
}
impl From<(&[f64], usize)> for Signals {
    fn from((data, n_step): (&[f64], usize)) -> Self {
        let n = data.len();
        data.iter()
            .enumerate()
            .fold(Signals::new(n, n_step), |s, (i, v)| {
                s.output_signal(i, Signal::Constant(*v))
            })
    }
}

impl Add for Signal {
    type Output = Signal;

    fn add(self, rhs: Self) -> Self::Output {
        if let Signal::Composite(mut signals) = self {
            signals.push(rhs);
            Signal::Composite(signals)
        } else {
            Signal::Composite(vec![self, rhs])
        }
    }
}

impl Update for Signals {
    fn update(&mut self) {
        if let Some(pb) = self.progress_bar.as_mut() {
            pb.progress.lock().unwrap().inc_and_draw(&pb.bar, 1)
        }
    }
}
impl<U: UniqueIdentifier<Data = Vec<f64>>> Write<U> for Signals {
    fn write(&mut self) -> Option<Arc<Data<U>>> {
        log::debug!("write {:?}", self.size);
        if self.step < self.n_step {
            let i = self.step;
            let data = self.signals.iter().map(|signal| signal.get(i)).collect();
            self.step += 1;
            Some(Arc::new(Data::new(data)))
        } else {
            None
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignalsError {
    #[error("Two many signal channels, should be only 1")]
    OneSignal,
}
pub struct OneSignal {
    pub signal: Signal,
    pub step: usize,
    pub n_step: usize,
    progress_bar: Option<ProgressBar>,
}
impl From<Signals> for Result<OneSignal, SignalsError> {
    fn from(mut signals: Signals) -> Self {
        if signals.signals.len() > 1 {
            Err(SignalsError::OneSignal)
        } else {
            Ok(OneSignal {
                signal: signals.signals.remove(0),
                step: signals.step,
                n_step: signals.n_step,
                progress_bar: signals.progress_bar,
            })
        }
    }
}
impl Update for OneSignal {
    fn update(&mut self) {
        if let Some(pb) = self.progress_bar.as_mut() {
            pb.progress.lock().unwrap().inc_and_draw(&pb.bar, 1)
        }
    }
}
impl<U: UniqueIdentifier<Data = f64>> Write<U> for OneSignal {
    fn write(&mut self) -> Option<Arc<Data<U>>> {
        if self.step < self.n_step {
            let i = self.step;
            let data = self.signal.get(i);
            self.step += 1;
            Some(Arc::new(Data::new(data)))
        } else {
            None
        }
    }
}
