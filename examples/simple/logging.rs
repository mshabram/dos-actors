use crate::{DifferentiatorToIntegrator, FilterToSink, SamplerToSink, SignalToFilter};
use dos_actors::{
    io::{Data, Read},
    Update,
};
use std::{ops::Deref, sync::Arc};

#[derive(Default)]
pub struct Logging(Vec<f64>);
impl Deref for Logging {
    type Target = Vec<f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Update for Logging {}
impl Read<SignalToFilter> for Logging {
    fn read(&mut self, data: Arc<Data<SignalToFilter>>) {
        self.0.push(**data);
    }
}
impl Read<FilterToSink> for Logging {
    fn read(&mut self, data: Arc<Data<FilterToSink>>) {
        self.0.push(**data);
    }
}
impl Read<SamplerToSink> for Logging {
    fn read(&mut self, data: Arc<Data<SamplerToSink>>) {
        self.0.push(**data);
    }
}
impl Read<DifferentiatorToIntegrator> for Logging {
    fn read(&mut self, data: Arc<Data<DifferentiatorToIntegrator>>) {
        self.0.push(**data);
    }
}
