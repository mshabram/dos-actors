use std::sync::Arc;

use crate::FilterToDifferentiator;
use dos_actors::{
    io::{Data, Read, Write},
    Update,
};
use uid_derive::UID;

#[derive(Default)]
pub struct Differentiator(f64, f64);
impl Update for Differentiator {}
impl Read<FilterToDifferentiator> for Differentiator {
    fn read(&mut self, data: Arc<Data<FilterToDifferentiator>>) {
        self.0 = **data;
    }
}
impl Read<IntegratorToDifferentiator> for Differentiator {
    fn read(&mut self, data: Arc<Data<IntegratorToDifferentiator>>) {
        self.1 = **data;
    }
}
#[derive(UID)]
#[uid(data = "f64")]
pub enum DifferentiatorToIntegrator {}
impl Write<DifferentiatorToIntegrator> for Differentiator {
    fn write(&mut self) -> Option<Arc<Data<DifferentiatorToIntegrator>>> {
        Some(Arc::new(Data::new(self.0 - self.1)))
    }
}

#[derive(Default)]
pub struct Integrator {
    gain: f64,
    mem: Vec<f64>,
}
impl Integrator {
    pub fn new(gain: f64, n_data: usize) -> Self {
        Self {
            gain,
            mem: vec![0f64; n_data],
        }
    }
    pub fn last(&self) -> Option<Vec<f64>> {
        Some(self.mem.clone())
    }
}
impl Update for Integrator {}
impl Read<DifferentiatorToIntegrator> for Integrator {
    fn read(&mut self, data: Arc<Data<DifferentiatorToIntegrator>>) {
        self.mem[0] += **data * self.gain;
    }
}
#[derive(UID)]
#[uid(data = "f64")]
pub enum IntegratorToDifferentiator {}
impl Write<IntegratorToDifferentiator> for Integrator {
    fn write(&mut self) -> Option<Arc<Data<IntegratorToDifferentiator>>> {
        self.last().map(|x| Arc::new(Data::new(x[0])))
    }
}
