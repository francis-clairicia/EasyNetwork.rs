use super::traits::{Producer, ProducerState};
use std::{
    borrow::Cow,
    fmt::{self},
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

pub fn from_coroutine<E, G>(coroutine: G) -> Pin<Box<FromCoroutineProducer<G>>>
where
    G: Coroutine<Yield = Vec<u8>, Return = Result<(), E>>,
{
    Box::pin(FromCoroutineProducer(coroutine))
}

pub struct FromCoroutineProducer<G>(G);

impl<G> fmt::Debug for FromCoroutineProducer<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromCoroutineProducer").finish()
    }
}

impl<E, G> Producer<'static> for FromCoroutineProducer<G>
where
    G: Coroutine<Yield = Vec<u8>, Return = Result<(), E>>,
{
    type Error = E;

    fn next(self: Pin<&mut Self>) -> ProducerState<'static, Self::Error> {
        // SAFETY: We are not moving out of the pinned field.
        match unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0) }.resume(()) {
            CoroutineState::Yielded(bytes) => ProducerState::Yielded(Cow::Owned(bytes)),
            CoroutineState::Complete(result) => ProducerState::Complete(result),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::serializers::producer::{Producer, ProducerState};

    use super::from_coroutine;
    use std::convert::Infallible;

    #[inline(always)]
    fn infallible<T>(v: T) -> Result<T, Infallible> {
        Ok(v)
    }

    #[test]
    fn it_works() {
        let mut producer = from_coroutine(
            #[coroutine]
            move || {
                yield b"pac".to_vec();
                yield b"ket\n".to_vec();
                infallible(())
            },
        );

        assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(b) if *b == *b"pac"));
        assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(b) if *b == *b"ket\n"));
        assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Ok(()))));
    }

    #[test]
    fn it_works_with_error() {
        let mut producer = from_coroutine(
            #[coroutine]
            move || {
                yield b"pac".to_vec();
                Err(42)
            },
        );

        assert!(matches!(producer.as_mut().next(), ProducerState::Yielded(b) if *b == *b"pac"));
        assert!(matches!(producer.as_mut().next(), ProducerState::Complete(Err(42))));
    }
}
