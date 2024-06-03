//! Simple functions for tests and examples.

use std::{hint::black_box, thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

/// Instrumented simple sync function
#[instrument(level = "trace", skip(repeats, sleep_micros))]
pub fn simple_fn_sync(repeats: usize, sleep_micros: u64) {
    #[instrument(level = "trace", skip(sleep_micros))]
    fn g_sync(i: usize, sleep_micros: u64) {
        // Simulated work
        thread::sleep(Duration::from_micros(sleep_micros * 2));
        black_box(i);
    }

    for i in 0..repeats {
        trace_span!("loop_body", foo = i % 2).in_scope(|| {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            thread::sleep(Duration::from_micros(sleep_micros * 3));

            g_sync(i, sleep_micros);
        });
    }
}

/// Uninstrumented simple sync function
pub fn simple_fn_sync_un(repeats: usize, sleep_micros: u64) {
    fn g_sync_un(i: usize, sleep_micros: u64) {
        // Simulated work
        thread::sleep(Duration::from_micros(sleep_micros * 2));
        black_box(i);
    }

    for i in 0..repeats {
        {
            // Simulated work
            thread::sleep(Duration::from_micros(sleep_micros * 3));

            g_sync_un(i, sleep_micros);
        };
    }
}

/// Instrumented simple async function
#[instrument(level = "trace", skip(repeats, sleep_micros))]
pub async fn simple_fn_async(repeats: usize, sleep_micros: u64) {
    #[instrument(level = "trace", skip(sleep_micros))]
    async fn g_async(i: usize, sleep_micros: u64) {
        // Simulated work
        tokio::time::sleep(Duration::from_micros(sleep_micros * 2)).await;
        black_box(i);
    }

    for i in 0..repeats {
        async {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            thread::sleep(Duration::from_micros(sleep_micros * 3));

            g_async(i, sleep_micros).await;
        }
        .instrument(trace_span!("loop_body", foo = i % 2))
        .await;
    }
}

/// Uninstrumented simple async function
pub async fn simple_fn_async_un(repeats: usize, sleep_micros: u64) {
    async fn g_async_un(i: usize, sleep_micros: u64) {
        // Simulated work
        tokio::time::sleep(Duration::from_micros(sleep_micros * 2)).await;
        black_box(i);
    }

    for i in 0..repeats {
        async {
            // Simulated work
            thread::sleep(Duration::from_micros(sleep_micros * 3));

            g_async_un(i, sleep_micros).await;
        }
        .await;
    }
}
