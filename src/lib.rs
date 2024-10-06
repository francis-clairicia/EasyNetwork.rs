#![cfg_attr(feature = "nightly", feature(coroutine_trait))]
#![cfg_attr(feature = "nightly", feature(coroutines))]
#![cfg_attr(feature = "nightly", feature(stmt_expr_attributes))]

// Public
pub mod converters;
pub mod errors;
pub mod protocols;
pub mod serializers;

// Private
mod constants;

pub(crate) mod sealed {
    pub trait Sealed {}
}
