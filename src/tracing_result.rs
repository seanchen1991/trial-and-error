//! A proposed extension to `std::result::Result` that makes use of `min_specialization` in order
//! to add a `Traced` trait that allows users to configure tracing behavior for any implementing
//! types. 
//!
//! # Examples
//!
//! ```rust
//! #![feature(min_specialization)]
//!
//! use std::panic::Location;
//! use trial_and_error::{Result, Err, Traced, TracedMarker};
//! 
//! // Make Strings traceable. Here we'll simply append a '!' for every propagation. In a more
//! // realistic use-case, we'd almost certainly want to make use of the `location` parameter.
//! struct NewString(String);
//!
//! impl<T: Into<String>> From<T> for NewString {
//!     fn from(s: T) -> Self {
//!         Self(s.into())
//!     }
//! }
//!
//! impl Traced for NewString {
//!     fn trace(&mut self, _location: &'static Location<'static>) {
//!         self.0.push('!');
//!     }
//! }
//!
//! impl TracedMarker for NewString {}
//!
//! fn baz() -> Result<(), NewString> {
//!     Err("Error".to_string())?
//! }
//!
//! fn boz() -> Result<(), NewString> {
//!     Err("Error")?
//! }
//! ```

use std::ops::{ControlFlow, FromResidual, Try};
use std::panic::Location;

/// Trait intended to allow users to configure tracing behavior 
/// on the implementing type.
pub trait Traced {
    /// Use the `trace` function to configure tracing behavior 
    /// of the implementing type.
    fn trace(&mut self, location: &'static Location<'static>);
}

#[rustc_specialization_trait]
pub trait TracedMarker: Traced {}

/// Dummy Result that implements `Traced`.
pub enum Result<T, E> {
    /// Ok variant of the Result.
    Ok(T),
    /// Error variant of the Result.
    Err(E),
}

pub use self::Result::Ok;
pub use self::Result::Err;

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<!, E>;

    fn from_output(output: T) -> Self {
        Ok(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Ok(t) => ControlFlow::Continue(t),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

// Default blanket FromResidual impl for Result
impl<T, E, F> FromResidual<Result<!, E>> for Result<T, F>
where 
    F: From<E>,
{
    default fn from_residual(residual: Result<!, E>) -> Self {
        match residual {
            Ok(_) => unreachable!(),
            Err(e) => Err(From::from(e)),
        }
    }
}

// Specialized FromResidual impl for types that implement `Traced`
impl<T, E, F> FromResidual<Result<!, E>> for Result<T, F>
where 
    F: From<E> + TracedMarker,
{
    #[track_caller]
    fn from_residual(residual: Result<!, E>) -> Self {
        match residual {
            Ok(_) => unreachable!(),
            Err(e) => {
                let mut f = F::from(e);
                f.trace(Location::caller());
                Err(f)
            }
        }
    }
}

