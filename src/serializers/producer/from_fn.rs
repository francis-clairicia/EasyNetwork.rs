use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    pin::Pin,
};

pub fn from_fn<'buf, E, F>(f: F) -> FromFnProducer<F>
where
    F: FnMut() -> ProducerState<'buf, E>,
{
    FromFnProducer(f)
}

pub struct FromFnProducer<F>(F);

impl<F> fmt::Debug for FromFnProducer<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromFnProducer").finish()
    }
}

impl<'buf, E, F> Producer<'buf> for FromFnProducer<F>
where
    F: FnMut() -> ProducerState<'buf, E>,
{
    type Error = E;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        // SAFETY: We are not moving out of the pinned field.
        (unsafe { &mut self.get_unchecked_mut().0 })()
    }
}
