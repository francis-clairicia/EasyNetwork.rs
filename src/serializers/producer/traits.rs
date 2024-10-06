use std::{borrow::Cow, pin::Pin};

pub enum ProducerState<'buf, E> {
    Yielded(Cow<'buf, [u8]>),
    Complete(Result<(), E>),
}

pub trait Producer<'buf> {
    type Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error>;
}

impl<'buf, E> ProducerState<'buf, E> {
    pub fn map<O: FnOnce(Cow<'buf, [u8]>) -> Cow<'buf, [u8]>>(self, op: O) -> ProducerState<'buf, E> {
        match self {
            ProducerState::Complete(result) => ProducerState::Complete(result),
            ProducerState::Yielded(bytes) => ProducerState::Yielded(op(bytes)),
        }
    }

    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> ProducerState<'buf, F> {
        match self {
            ProducerState::Complete(result) => ProducerState::Complete(result.map_err(op)),
            ProducerState::Yielded(bytes) => ProducerState::Yielded(bytes),
        }
    }
}

impl<'buf, E> From<Result<Vec<u8>, E>> for ProducerState<'buf, E> {
    #[inline]
    fn from(result: Result<Vec<u8>, E>) -> Self {
        match result {
            Ok(bytes) => ProducerState::Yielded(Cow::Owned(bytes)),
            Err(e) => ProducerState::Complete(Err(e)),
        }
    }
}

impl<'buf, E> From<Result<Cow<'buf, [u8]>, E>> for ProducerState<'buf, E> {
    #[inline]
    fn from(result: Result<Cow<'buf, [u8]>, E>) -> Self {
        match result {
            Ok(bytes) => ProducerState::Yielded(bytes),
            Err(e) => ProducerState::Complete(Err(e)),
        }
    }
}

impl<'buf, P: ?Sized + Producer<'buf>> Producer<'buf> for Pin<&mut P> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        P::next((*self).as_mut())
    }
}

impl<'buf, P: ?Sized + Producer<'buf> + Unpin> Producer<'buf> for &mut P {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        P::next(Pin::new(&mut *self))
    }
}

impl<'buf, P: ?Sized + Producer<'buf>> Producer<'buf> for Pin<Box<P>> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        P::next((*self).as_mut())
    }
}

impl<'buf, P: ?Sized + Producer<'buf> + Unpin> Producer<'buf> for Box<P> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        P::next(Pin::new(&mut *self))
    }
}
