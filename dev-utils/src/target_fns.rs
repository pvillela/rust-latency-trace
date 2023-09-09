use std::{thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

#[instrument(level = "trace")]
async fn f() {
    let mut foo: u64 = 1;

    for i in 0..8 {
        log::trace!("Before outer_async_span");

        async {
            trace_span!("sync_span_1").in_scope(|| {
                thread::sleep(Duration::from_millis(13));
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            foo += 1;
            log::trace!("Before inner_async_span");
            async {
                {
                    let span = trace_span!("sync_span_2");
                    let _enter = span.enter();
                    thread::sleep(Duration::from_millis(12));
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            .instrument(trace_span!("inner_async_span", foo = i % 2))
            .await;
        }
        .instrument(trace_span!("outer_async_span", foo = i % 2, bar = i % 4))
        .await
    }
}

pub async fn target_fn() {
    let h1 = tokio::spawn(async { f().await }.instrument(trace_span!("root_async_1")));
    let h2 = tokio::spawn(async { f().await }.instrument(trace_span!("root_async_2")));
    _ = h1.await;
    _ = h2.await;
}
