mod from_fn;
mod traits;

#[cfg(feature = "nightly")]
mod from_coroutine;

pub use from_fn::{from_fn, FromFnConsumer};
pub use traits::{Consumer, ConsumerState};

#[cfg(feature = "nightly")]
pub use from_coroutine::{from_coroutine, FromCoroutineConsumer};
