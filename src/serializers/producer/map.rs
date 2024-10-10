use super::traits::{Producer, ProducerState};
use std::{
    fmt::{self},
    pin::Pin,
};

pub fn map<'buf, WrappedProducer: ?Sized, Func>(
    producer: Pin<Box<WrappedProducer>>,
    func: Func,
) -> Pin<Box<MapProducer<WrappedProducer, Func>>>
where
    MapProducer<WrappedProducer, Func>: Producer<'buf>,
{
    Box::pin(MapProducer {
        producer,
        func: Some(func),
    })
}

pub struct MapProducer<WrappedProducer: ?Sized, Func> {
    producer: Pin<Box<WrappedProducer>>,
    func: Option<Func>,
}

impl<WrappedProducer: ?Sized, Func> fmt::Debug for MapProducer<WrappedProducer, Func> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapProducer").finish()
    }
}

impl<'buf, 'producer, WrappedProducer, Func, Error> Producer<'buf> for MapProducer<WrappedProducer, Func>
where
    'buf: 'producer,
    WrappedProducer: ?Sized + Producer<'buf> + 'producer,
    Func: Unpin + FnOnce(Result<(), WrappedProducer::Error>) -> Result<(), Error>,
{
    type Error = Error;

    fn next(self: Pin<&mut Self>) -> ProducerState<'buf, Self::Error> {
        let this = self.get_mut();

        match this.producer.as_mut().next() {
            ProducerState::Yielded(bytes) => ProducerState::Yielded(bytes),
            ProducerState::Complete(result) => {
                let map_fn = this.func.take().expect("a producer should not be used after completion");
                let result = map_fn(result);

                ProducerState::Complete(result)
            }
        }
    }
}
