pub mod diagnostics;
mod frontend;
mod standalone;
pub use frontend::parse;
pub use standalone::lower;

#[cfg(any(feature = "native", feature = "browser"))]
mod backend;
#[cfg(feature = "native")]
mod entry;

#[cfg(feature = "browser")]
pub mod cross;
#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "native")]
pub mod runtime;
