//! Elaborate functions for tests and examples.

#![allow(clippy::disallowed_names)]

use crate::gater::Gater;
use std::{sync::Arc, thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

pub const PROBE_GATE_F_PROCEED: u8 = 0;
pub const PROBE_GATE_F1_PROBE_READY: u8 = 1;
pub const PROBE_GATE_F2_PROBE_READY: u8 = 2;

#[instrument(level = "trace", skip(f_instance, probe_gater))]
async fn f(f_instance: u8, probe_gater: Option<Arc<Gater>>) {
    let mut foo: u64 = 1;

    for i in 0..8 {
        log::trace!("Before outer_async_span");

        if i == 4 {
            if let Some(probe_gater) = probe_gater.clone() {
                probe_gater.open(f_instance);
                probe_gater.wait_for_async(PROBE_GATE_F_PROCEED).await;
            }
        }

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

pub async fn elab_fn_async_gated(probe_gater: Option<Arc<Gater>>) {
    let h1 = {
        let probe_gater = probe_gater.clone();
        tokio::spawn(
            async { f(1, probe_gater).await }.instrument(trace_span!("root_async_1", foo = 1)),
        )
    };
    let h2 = {
        tokio::spawn(
            async { f(2, probe_gater).await }.instrument(trace_span!("root_async_2", bar = 2)),
        )
    };
    h1.await.unwrap();
    h2.await.unwrap();
}

pub async fn elab_fn_async() {
    elab_fn_async_gated(None).await
}
