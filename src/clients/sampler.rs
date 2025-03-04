use crate::{
    io::{Data, Read, UniqueIdentifier, Write},
    Update,
};
use std::{marker::PhantomData, sync::Arc};

/// Rate transitionner
#[derive(Debug)]
pub struct Sampler<T, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T> = U> {
    input: Arc<Data<U>>,
    output: PhantomData<V>,
}
impl<T, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T>> Sampler<T, U, V> {
    /// Creates a new sampler with initial condition
    pub fn new(init: T) -> Self {
        Self {
            input: Arc::new(Data::new(init)),
            output: PhantomData,
        }
    }
}
impl<T: Default, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T>> Default
    for Sampler<T, U, V>
{
    fn default() -> Self {
        Self {
            input: Arc::new(Data::new(T::default())),
            output: PhantomData,
        }
    }
}
impl<T, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T>> Update for Sampler<T, U, V> {}
impl<T, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T>> Read<U> for Sampler<T, U, V> {
    fn read(&mut self, data: Arc<Data<U>>) {
        self.input = data;
    }
}
impl<T: Clone, U: UniqueIdentifier<Data = T>, V: UniqueIdentifier<Data = T>> Write<V>
    for Sampler<T, U, V>
{
    fn write(&mut self) -> Option<Arc<Data<V>>> {
        Some(Arc::new(Data::new((**self.input).clone())))
    }
}
