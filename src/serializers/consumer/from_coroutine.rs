use super::traits::{Consumer, ConsumerState};
use std::{
    borrow::Cow,
    fmt::{self},
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

pub fn from_coroutine<T, E, G>(coroutine: G) -> Pin<Box<FromCoroutineConsumer<G>>>
where
    G: Coroutine<Vec<u8>, Yield = (), Return = Result<(T, Vec<u8>), (E, Vec<u8>)>>,
{
    Box::pin(FromCoroutineConsumer(coroutine))
}

pub struct FromCoroutineConsumer<G>(G);

impl<F> fmt::Debug for FromCoroutineConsumer<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromCoroutineConsumer").finish()
    }
}

impl<T, E, G> Consumer for FromCoroutineConsumer<G>
where
    G: Coroutine<Vec<u8>, Yield = (), Return = Result<(T, Vec<u8>), (E, Vec<u8>)>>,
{
    type Item = T;
    type Error = E;

    fn consume<'buf>(self: Pin<&mut Self>, buf: &'buf [u8]) -> ConsumerState<'buf, Self::Item, Self::Error> {
        let buf = buf.to_vec();

        // SAFETY: We are not moving out of the pinned field.
        match unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0) }.resume(buf) {
            CoroutineState::Yielded(()) => ConsumerState::InputNeeded,
            CoroutineState::Complete(Ok((item, remainder))) => ConsumerState::Complete(Ok(item), Cow::from(remainder)),
            CoroutineState::Complete(Err((error, remainder))) => ConsumerState::Complete(Err(error), Cow::from(remainder)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::LimitOverrunError;
    use crate::serializers::consumer::{Consumer, ConsumerState};
    use crate::serializers::tools::{BufferedStreamReader, StreamReadOwnedError};

    use super::from_coroutine;
    use std::num::NonZeroUsize;

    #[test]
    fn it_works() {
        let mut consumer = from_coroutine(
            #[coroutine]
            move |buf: Vec<u8>| {
                let mut reader = BufferedStreamReader::from(buf);
                let limit = NonZeroUsize::new(128).unwrap();
                let keep_end = true;

                loop {
                    match reader.read_until(b"\n", limit, keep_end) {
                        Ok(data) => break Ok((data, reader.read_all())),
                        Err(StreamReadOwnedError::InputNeeded) => {
                            let buf = yield;
                            reader.fill_buf(&buf);
                        }
                        Err(StreamReadOwnedError::Error(e, remainder)) => return Err((e, remainder)),
                    }
                }
            },
        );

        assert!(matches!(consumer.as_mut().consume(b"pac"), ConsumerState::InputNeeded));
        assert!(matches!(consumer.as_mut().consume(b"k"), ConsumerState::InputNeeded));
        assert!(
            matches!(consumer.as_mut().consume(b"et\n"), ConsumerState::Complete(Ok(b), r) if *b == *b"packet\n" && *r == *b"")
        );
    }

    #[test]
    fn it_works_with_error() {
        let mut consumer = from_coroutine(
            #[coroutine]
            move |initial: Vec<u8>| {
                let mut reader = BufferedStreamReader::from(initial);
                let limit = NonZeroUsize::new(3).unwrap();
                let keep_end = true;

                loop {
                    match reader.read_until(b"\n", limit, keep_end) {
                        Ok(data) => break Ok((data, reader.read_all())),
                        Err(StreamReadOwnedError::InputNeeded) => {
                            let buf = yield;
                            reader.fill_buf(&buf);
                        }
                        Err(StreamReadOwnedError::Error(e, remainder)) => return Err((e, remainder)),
                    }
                }
            },
        );

        assert!(matches!(consumer.as_mut().consume(b"pa"), ConsumerState::InputNeeded));
        assert!(matches!(
            consumer.as_mut().consume(b"cket"),
            ConsumerState::Complete(Err(LimitOverrunError { .. }), _)
        ));
    }
}
