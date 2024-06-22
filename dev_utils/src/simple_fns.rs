//! Simple functions for tests and examples.

use futures::future::join_all;
use std::{hint::black_box, thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

/// Instrumented simple sync function
#[instrument(level = "trace", skip(nrepeats, sleep_micros))]
pub fn simple_fn_sync(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    #[instrument(level = "trace", skip(sleep_micros))]
    fn g_sync(i: usize, sleep_micros: u64) {
        // Simulated work
        thread::sleep(Duration::from_micros(sleep_micros * 2));
        black_box(i);
    }

    let f = move || {
        trace_span!("f").in_scope(|| {
            for i in 0..nrepeats {
                trace_span!("loop_body", foo = i % 2).in_scope(|| {
                    // Simulated work
                    thread::sleep(Duration::from_micros(sleep_micros * 3));

                    g_sync(i, sleep_micros);
                });
            }
        });
    };

    let hs = (0..ntasks).map(|_| thread::spawn(f)).collect::<Vec<_>>();
    f();
    hs.into_iter().for_each(|h| h.join().unwrap());
}

/// Uninstrumented simple sync function
pub fn simple_fn_sync_un(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    fn g_sync_un(i: usize, sleep_micros: u64) {
        // Simulated work
        thread::sleep(Duration::from_micros(sleep_micros * 2));
        black_box(i);
    }

    let f = move || {
        for i in 0..nrepeats {
            {
                // Simulated work
                thread::sleep(Duration::from_micros(sleep_micros * 3));

                g_sync_un(i, sleep_micros);
            };
        }
    };

    let hs = (0..ntasks).map(|_| thread::spawn(f)).collect::<Vec<_>>();
    f();
    hs.into_iter().for_each(|h| h.join().unwrap());
}

/// Instrumented simple async function
#[instrument(level = "trace", skip(nrepeats, sleep_micros))]
pub async fn simple_fn_async(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    #[instrument(level = "trace", skip(sleep_micros))]
    async fn g_async(i: usize, sleep_micros: u64) {
        // Simulated work
        tokio::time::sleep(Duration::from_micros(sleep_micros * 2)).await;
        black_box(i);
    }

    let f = || {
        async move {
            for i in 0..nrepeats {
                async {
                    // Simulated work
                    thread::sleep(Duration::from_micros(sleep_micros * 3));

                    g_async(i, sleep_micros).await;
                }
                .instrument(trace_span!("loop_body", foo = i % 2))
                .await;
            }
        }
        .instrument(trace_span!("f"))
    };

    let hs = (0..ntasks).map(|_| tokio::spawn(f())).collect::<Vec<_>>();
    f().await;
    join_all(hs).await.into_iter().for_each(|r| r.unwrap());
}

/// Uninstrumented simple async function
pub async fn simple_fn_async_un(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    async fn g_async_un(i: usize, sleep_micros: u64) {
        // Simulated work
        tokio::time::sleep(Duration::from_micros(sleep_micros * 2)).await;
        black_box(i);
    }

    let f = || {
        async move {
            for i in 0..nrepeats {
                async {
                    // Simulated work
                    thread::sleep(Duration::from_micros(sleep_micros * 3));

                    g_async_un(i, sleep_micros).await;
                }
                .await;
            }
        }
    };

    let hs = (0..ntasks).map(|_| tokio::spawn(f())).collect::<Vec<_>>();
    f().await;
    join_all(hs).await.into_iter().for_each(|r| r.unwrap());
}
