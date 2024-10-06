use std::pin::Pin;

pub enum ProducerState<E> {
    Yielded(Vec<u8>),
    Complete(Result<(), E>),
}

pub trait Producer {
    type Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<Self::Error>;
}

impl<E> ProducerState<E> {
    pub fn map<O: FnOnce(Vec<u8>) -> Vec<u8>>(self, op: O) -> ProducerState<E> {
        match self {
            ProducerState::Complete(result) => ProducerState::Complete(result),
            ProducerState::Yielded(bytes) => ProducerState::Yielded(op(bytes)),
        }
    }

    pub fn map_err<F, O: FnOnce(E) -> F>(self, op: O) -> ProducerState<F> {
        match self {
            ProducerState::Complete(result) => ProducerState::Complete(result.map_err(op)),
            ProducerState::Yielded(bytes) => ProducerState::Yielded(bytes),
        }
    }
}

impl<E> From<Result<Vec<u8>, E>> for ProducerState<E> {
    #[inline]
    fn from(result: Result<Vec<u8>, E>) -> Self {
        match result {
            Ok(bytes) => ProducerState::Yielded(bytes),
            Err(e) => ProducerState::Complete(Err(e)),
        }
    }
}

impl<P: ?Sized + Producer> Producer for Pin<&mut P> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<Self::Error> {
        P::next((*self).as_mut())
    }
}

impl<P: ?Sized + Producer + Unpin> Producer for &mut P {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<Self::Error> {
        P::next(Pin::new(&mut *self))
    }
}

impl<P: ?Sized + Producer> Producer for Pin<Box<P>> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<Self::Error> {
        P::next((*self).as_mut())
    }
}

impl<P: ?Sized + Producer + Unpin> Producer for Box<P> {
    type Error = P::Error;

    fn next(mut self: Pin<&mut Self>) -> ProducerState<Self::Error> {
        P::next(Pin::new(&mut *self))
    }
}
