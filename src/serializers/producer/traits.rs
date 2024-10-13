use std::{borrow::Cow, pin::Pin};

pub enum ProducerState<'buf, E> {
    Yielded(Cow<'buf, [u8]>),
    Complete(Result<(), E>),
}

pub trait Producer<'buf> {
    type Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error>;
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
