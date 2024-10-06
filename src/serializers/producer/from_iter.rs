use super::traits::{Producer, ProducerState};
use std::borrow::Cow;
use std::fmt;
use std::{convert::Infallible, pin::Pin};

pub fn from_iter<I>(iterable: impl IntoIterator<IntoIter = I>) -> Pin<Box<FromIterProducer<I>>>
where
    FromIterProducer<I>: Producer<'static>,
{
    Box::pin(FromIterProducer(iterable.into_iter()))
}

pub struct FromIterProducer<I>(I);

impl<F> fmt::Debug for FromIterProducer<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromIterProducer").finish()
    }
}

impl<I: Iterator<Item = Vec<u8>>> Producer<'static> for FromIterProducer<I> {
    type Error = Infallible;

    fn next(self: Pin<&mut Self>) -> ProducerState<'static, Self::Error> {
        // SAFETY: We are not moving out of the pinned field.
        match (unsafe { &mut self.get_unchecked_mut().0 }).next() {
            Some(bytes) => ProducerState::Yielded(Cow::Owned(bytes)),
            None => ProducerState::Complete(Ok(())),
        }
    }
}
