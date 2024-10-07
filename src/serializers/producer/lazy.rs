use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    marker::PhantomData,
    pin::Pin,
};

pub fn lazy<'packet, Initializer, WrappedProducer: ?Sized, Error>(
    initializer: Initializer,
) -> Pin<Box<LazyProducer<Initializer, WrappedProducer, Error>>>
where
    LazyProducer<Initializer, WrappedProducer, Error>: Producer<'packet>,
{
    Box::pin(LazyProducer {
        state: LazyProducerState::Initialization(initializer),
        _marker: PhantomData,
    })
}

enum LazyProducerState<Initializer, WrappedProducer: ?Sized> {
    Initialization(Initializer),
    Available(Pin<Box<WrappedProducer>>),
    Complete,
}

pub struct LazyProducer<Initializer, WrappedProducer: ?Sized, Error> {
    state: LazyProducerState<Initializer, WrappedProducer>,
    _marker: PhantomData<Error>,
}

impl<Initializer, WrappedProducer: ?Sized, Error> fmt::Debug for LazyProducer<Initializer, WrappedProducer, Error> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LazyProducer").finish()
    }
}

impl<'packet, 'producer, Initializer, WrappedProducer, Error> Producer<'packet>
    for LazyProducer<Initializer, WrappedProducer, Error>
where
    'packet: 'producer,
    WrappedProducer: ?Sized + Producer<'packet, Error = Error> + 'producer,
    Initializer: FnOnce() -> Result<Pin<Box<WrappedProducer>>, Error>,
{
    type Error = Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'packet, Self::Error> {
        use std::mem;
        use LazyProducerState::*;

        let this = unsafe { self.get_unchecked_mut() };

        match mem::replace(&mut this.state, Complete) {
            Initialization(initializer) => match initializer() {
                Ok(producer) => {
                    this.state = Available(producer);

                    unsafe { Pin::new_unchecked(this) }.next()
                }
                Err(error) => {
                    this.state = Complete;

                    ProducerState::Complete(Err(error))
                }
            },
            Available(mut wrapper) => match wrapper.as_mut().next() {
                ProducerState::Yielded(bytes) => {
                    this.state = Available(wrapper);

                    ProducerState::Yielded(bytes)
                }
                ProducerState::Complete(result) => {
                    this.state = Complete;

                    ProducerState::Complete(result)
                }
            },
            Complete => panic!("producer used after completion"),
        }
    }
}
