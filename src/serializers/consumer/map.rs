use super::traits::{Consumer, ConsumerState};
use std::{
    fmt::{self},
    pin::Pin,
};

pub fn map<WrappedConsumer: ?Sized, Func>(
    consumer: Pin<Box<WrappedConsumer>>,
    func: Func,
) -> Pin<Box<MapConsumer<WrappedConsumer, Func>>>
where
    MapConsumer<WrappedConsumer, Func>: Consumer,
{
    Box::pin(MapConsumer {
        consumer,
        func: Some(func),
    })
}

pub struct MapConsumer<WrappedConsumer: ?Sized, Func> {
    consumer: Pin<Box<WrappedConsumer>>,
    func: Option<Func>,
}

impl<WrappedConsumer: ?Sized, Func> fmt::Debug for MapConsumer<WrappedConsumer, Func> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapConsumer").finish()
    }
}

impl<WrappedConsumer, Func, T, E> Consumer for MapConsumer<WrappedConsumer, Func>
where
    WrappedConsumer: ?Sized + Consumer,
    Func: Unpin + FnOnce(Result<WrappedConsumer::Item, WrappedConsumer::Error>) -> Result<T, E>,
{
    type Item = T;

    type Error = E;

    fn consume<'buf>(self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        let this = self.get_mut();
        match this.consumer.as_mut().consume(buf) {
            ConsumerState::InputNeeded => ConsumerState::InputNeeded,
            ConsumerState::Complete(result, remainder) => {
                let map_fn = this.func.take().expect("a consumer should not be called after completion");
                let result = map_fn(result);

                ConsumerState::Complete(result, remainder)
            }
        }
    }
}
