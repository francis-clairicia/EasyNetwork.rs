use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    pin::Pin,
};

pub fn lazy<'buf, Initializer, WrappedProducer: ?Sized>(
    initializer: Initializer,
) -> Pin<Box<LazyProducer<Initializer, WrappedProducer>>>
where
    LazyProducer<Initializer, WrappedProducer>: Producer<'buf>,
{
    Box::pin(LazyProducer {
        state: LazyProducerState::Initialization(initializer),
    })
}

enum LazyProducerState<Initializer, WrappedProducer: ?Sized> {
    Initialization(Initializer),
    Available(Pin<Box<WrappedProducer>>),
    Complete,
}

pub struct LazyProducer<Initializer, WrappedProducer: ?Sized> {
    state: LazyProducerState<Initializer, WrappedProducer>,
}

impl<Initializer, WrappedProducer: ?Sized> fmt::Debug for LazyProducer<Initializer, WrappedProducer> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LazyProducer").finish()
    }
}

impl<'buf, 'producer, Initializer, WrappedProducer, Error> Producer<'buf> for LazyProducer<Initializer, WrappedProducer>
where
    'buf: 'producer,
    WrappedProducer: ?Sized + Producer<'buf, Error = Error> + 'producer,
    Initializer: Unpin + FnOnce() -> Result<Pin<Box<WrappedProducer>>, Error>,
{
    type Error = Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        use std::mem;
        use LazyProducerState::*;

        let this = self.get_mut();

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
