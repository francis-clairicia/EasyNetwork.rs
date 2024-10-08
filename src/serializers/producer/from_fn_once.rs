use super::traits::{Producer, ProducerState};
use std::{
    borrow::Cow,
    fmt::{self},
    mem::{self},
    pin::Pin,
};

pub fn from_fn_once<'buf, F>(f: F) -> Pin<Box<FromFnOnceProducer<F>>>
where
    FromFnOnceProducer<F>: Producer<'buf>,
{
    Box::pin(FromFnOnceProducer(FromFnOnceProducerState::Start(f)))
}

pub struct FromFnOnceProducer<F>(FromFnOnceProducerState<F>);

enum FromFnOnceProducerState<F> {
    Start(F),
    Succeeded,
    Complete,
}

impl<F> fmt::Debug for FromFnOnceProducer<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromFnOnceProducer").finish()
    }
}

impl<'buf, E, F> Producer<'buf> for FromFnOnceProducer<F>
where
    F: Unpin + FnOnce() -> Result<Cow<'buf, [u8]>, E>,
{
    type Error = E;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        let this = self.get_mut();
        match mem::replace(&mut this.0, FromFnOnceProducerState::Complete) {
            FromFnOnceProducerState::Start(next_fn) => next_fn().inspect(|_| this.0 = FromFnOnceProducerState::Succeeded).into(),
            FromFnOnceProducerState::Succeeded => ProducerState::Complete(Ok(())),
            FromFnOnceProducerState::Complete => panic!("producer used after completion"),
        }
    }
}
