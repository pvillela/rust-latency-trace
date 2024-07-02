#![doc = include_str!("lib1.md")]
//!
//! ### Simple sync example
//!
//! ```rust
#![doc = include_str!("../examples/doc_sync.rs")]
//! ```
//!
//! ### Simple async example
//!
//! ```rust
#![doc = include_str!("../examples/doc_async.rs")]
//! ```
//!
//! ### Simple sync probed example
//!
//! ```rust
#![doc = include_str!("../examples/doc_sync_probed.rs")]
//! ```
//!
//! ### Simple async probed example
//!
//! ```rust
#![doc = include_str!("../examples/doc_async_probed.rs")]
//! ```
//!
#![doc = include_str!("lib2.md")]
#![deny(clippy::unwrap_used)]

mod lt_collect;
mod lt_collect_g;
mod lt_refine;
mod lt_refine_g;
mod tlc_param;

mod lt_pub;
mod lt_pub_g;
pub use lt_pub::*;

mod span_groupers;
pub use span_groupers::*;

mod summary_stats;
pub use summary_stats::*;

mod probed_trace;
mod probed_trace_g;
pub use probed_trace::*;

mod wrapper;
pub use wrapper::*;

#[cfg(feature = "dev")]
#[doc(hidden)]
pub mod bench_support;
