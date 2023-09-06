use crate::{default_span_grouper, Latencies};
use std::{future::Future, sync::Arc};
use tracing_core::span::Attributes;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
fn measure_latencies_priv(
    span_grouper: impl Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'static,
    f: impl FnOnce() + Send + 'static,
) -> Latencies {
    let mut latencies = Latencies::new(Arc::new(span_grouper));
    Registry::default().with(latencies.clone()).init();
    f();
    latencies.control.ensure_tls_dropped();
    latencies.update_info();
    latencies
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies(f: impl FnOnce() -> () + Send + 'static) -> Latencies {
    measure_latencies_priv(default_span_grouper, f)
}

/// Measures latencies of spans in `f`.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_with_custom_grouping(
    span_grouper: impl Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'static,
    f: impl FnOnce() -> () + Send + 'static,
) -> Latencies {
    measure_latencies_priv(span_grouper, f)
}

/// Measures latencies of spans in async function `f` running on the [tokio] runtime.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_tokio<F>(f: impl FnOnce() -> F + Send + 'static) -> Latencies
where
    F: Future<Output = ()> + Send,
{
    measure_latencies(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                f().await;
            });
    })
}

/// Measures latencies of spans in async function `f` running on the [tokio] runtime.
/// May only be called once per process and will panic if called more than once.
pub fn measure_latencies_with_custom_grouping_tokio<F>(
    span_grouper: impl Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'static,
    f: impl FnOnce() -> F + Send + 'static,
) -> Latencies
where
    F: Future<Output = ()> + Send,
{
    measure_latencies_with_custom_grouping(span_grouper, || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                f().await;
            });
    })
}
