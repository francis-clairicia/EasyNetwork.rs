use std::{borrow::Cow, pin::Pin};

#[derive(Debug, Clone)]
pub enum ConsumerState<'buf, T, E> {
    InputNeeded,
    Complete(Result<T, E>, Cow<'buf, [u8]>),
}

pub trait Consumer {
    type Item;
    type Error;

    fn consume<'buf>(self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error>;
}

impl<C: ?Sized + Consumer> Consumer for Pin<&mut C> {
    type Item = C::Item;
    type Error = C::Error;

    fn consume<'buf>(mut self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        C::consume((*self).as_mut(), buf)
    }
}

impl<C: ?Sized + Consumer + Unpin> Consumer for &mut C {
    type Item = C::Item;
    type Error = C::Error;

    fn consume<'buf>(mut self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        C::consume(Pin::new(&mut *self), buf)
    }
}

impl<C: ?Sized + Consumer> Consumer for Pin<Box<C>> {
    type Item = C::Item;
    type Error = C::Error;

    fn consume<'buf>(mut self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        C::consume((*self).as_mut(), buf)
    }
}

impl<C: ?Sized + Consumer + Unpin> Consumer for Box<C> {
    type Item = C::Item;
    type Error = C::Error;

    fn consume<'buf>(mut self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        C::consume(Pin::new(&mut *self), buf)
    }
}
