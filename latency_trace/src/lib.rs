#![doc = include_str!("lib1.md")]
//!
//! ### Simple sync Subscriber example
//!
//! ```rust
#![doc = include_str!("../examples/doc_sync.rs")]
//! ```
//!
//! ### Simple async Subscriber example
//!
//! ```rust
#![doc = include_str!("../examples/doc_async.rs")]
//! ```
//!
//! ### Simple sync probed Subscriber example
//!
//! ```rust
#![doc = include_str!("../examples/doc_sync_probed.rs")]
//! ```
//!
//! ### Simple async probed Subscriber example
//!
//! ```rust
#![doc = include_str!("../examples/doc_async_probed.rs")]
//! ```
//!
//! ### Simple sync Layer example
//!
//! ```rust
#![doc = include_str!("../examples/doc_sync_layered.rs")]
//! ```
//!
//! ### Other Layer examples
//! The above Subscriber examples can be easily modified based on the above Layer example to create analogous layered examples.
//! See also `test_layer_sync.rs` and `test_layer_async.rs` in the GitHub repo under the `dev_support/tests` directory.
//!
#![doc = include_str!("lib2.md")]
#![deny(clippy::unwrap_used)]

mod lt_collect_g;
mod lt_refine_g;
mod lt_report_g;
mod probed_trace;
mod tlc_param;

mod lt_pub;
pub use lt_pub::*;

mod span_groupers;
pub use span_groupers::*;

mod summary_stats;
pub use summary_stats::*;

mod wrapper;
pub use wrapper::*;

#[cfg(feature = "tokio")]
mod lt_report_g_tokio;

#[cfg(feature = "tokio")]
mod lt_pub_tokio;

#[cfg(feature = "dev")]
#[doc(hidden)]
pub mod bench_support;

#[cfg(feature = "dev")]
#[doc(hidden)]
pub mod bench_support_j;
