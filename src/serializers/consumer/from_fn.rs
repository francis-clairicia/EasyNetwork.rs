use super::traits::{Consumer, ConsumerState};
use std::{
    fmt::{self},
    pin::Pin,
};

pub fn from_fn<F>(f: F) -> Pin<Box<FromFnConsumer<F>>>
where
    FromFnConsumer<F>: Consumer,
{
    Box::pin(FromFnConsumer(f))
}

pub struct FromFnConsumer<F>(F);

impl<F> fmt::Debug for FromFnConsumer<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromFnConsumer").finish()
    }
}

impl<T, E, F> Consumer for FromFnConsumer<F>
where
    F: for<'buf> FnMut(&'buf [u8]) -> ConsumerState<'buf, T, E>,
{
    type Item = T;
    type Error = E;

    fn consume<'buf>(self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        // SAFETY: We are not moving out of the pinned field.
        (unsafe { &mut self.get_unchecked_mut().0 })(buf)
    }
}
