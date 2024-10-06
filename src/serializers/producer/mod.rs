mod from_fn;
mod from_fn_once;
mod from_iter;
mod traits;

#[cfg(feature = "nightly")]
mod from_coroutine;

pub use from_fn::{from_fn, FromFnProducer};
pub use from_fn_once::{from_fn_once, FromFnOnceProducer};
pub use from_iter::{from_iter, FromIterProducer};
pub use traits::{Producer, ProducerState};

#[cfg(feature = "nightly")]
pub use from_coroutine::{from_coroutine, FromCoroutineProducer};
