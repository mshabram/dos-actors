//! [Actor](crate::Actor)s [Input]/[Output]

use crate::Result;
use flume::{Receiver, Sender};
use futures::future::join_all;
use std::{ops::Deref, sync::Arc};

/// [Input]/[Output] data
///
/// `N` is the data transfer rate
#[derive(Debug, Default)]
pub struct Data<T: Default, const N: usize>(pub T);
impl<T, const N: usize> Deref for Data<T, N>
where
    T: Default,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T, const N: usize> From<&Data<Vec<T>, N>> for Vec<T>
where
    T: Default + Clone,
{
    fn from(data: &Data<Vec<T>, N>) -> Self {
        data.to_vec()
    }
}
impl<T, const N: usize> From<Vec<T>> for Data<Vec<T>, N>
where
    T: Default,
{
    fn from(u: Vec<T>) -> Self {
        Data(u)
    }
}

pub(crate) type S<T, const N: usize> = Arc<Data<T, N>>;

/// [Actor](crate::Actor)s input
#[derive(Debug)]
pub struct Input<T: Default, const N: usize> {
    pub data: S<T, N>,
    pub rx: Receiver<S<T, N>>,
}
impl<T, const N: usize> Input<T, N>
where
    T: Default,
{
    pub fn new(rx: Receiver<S<T, N>>) -> Self {
        Self {
            data: Default::default(),
            rx,
        }
    }
    pub async fn recv(&mut self) -> Result<&mut Self> {
        self.data = self.rx.recv_async().await?;
        Ok(self)
    }
}
impl<T, const N: usize> From<&Input<Vec<T>, N>> for Vec<T>
where
    T: Default + Clone,
{
    fn from(input: &Input<Vec<T>, N>) -> Self {
        input.data.as_ref().into()
    }
}
/// [Actor](crate::Actor)s output
#[derive(Debug)]
pub struct Output<T: Default, const N: usize> {
    pub data: S<T, N>,
    pub tx: Vec<Sender<S<T, N>>>,
}
impl<T: Default, const N: usize> Output<T, N> {
    pub fn new(tx: Vec<Sender<S<T, N>>>) -> Self {
        Self {
            data: Default::default(),
            tx,
        }
    }
    pub async fn send(&self) -> Result<&Self> {
        let futures: Vec<_> = self
            .tx
            .iter()
            .map(|tx| tx.send_async(self.data.clone()))
            .collect();
        join_all(futures)
            .await
            .into_iter()
            .collect::<std::result::Result<Vec<()>, flume::SendError<_>>>()
            .map_err(|_| flume::SendError(()))?;
        Ok(self)
    }
}

pub fn channels<T, const N: usize>(n_pairs: usize) -> (Output<T, N>, Vec<Input<T, N>>)
where
    T: Default,
{
    let mut txs = vec![];
    let mut inputs = vec![];
    for _ in 0..n_pairs {
        let (tx, rx) = flume::bounded::<S<T, N>>(1);
        txs.push(tx);
        inputs.push(Input::new(rx));
    }
    (Output::new(txs), inputs)
}
pub fn channel<T, const N: usize>() -> (Output<T, N>, Input<T, N>)
where
    T: Default,
{
    let (output, mut inputs) = channels(1);
    (output, inputs.pop().unwrap())
}
