#![doc = include_str!("../../readme0.md")]
//!
//! ## Example usage
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
#![doc = include_str!("../../readme2.md")]

mod core_internals_post;
mod core_internals_pre;

mod core_internals_pub;
pub use core_internals_pub::*;

mod span_groupers;
pub use span_groupers::*;

mod latency_trace;
pub use latency_trace::*;

mod summary_stats;
pub use summary_stats::*;

mod probed_trace;
pub use probed_trace::*;

mod wrapper;
pub use wrapper::*;
