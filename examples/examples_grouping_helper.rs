use log;
use std::{thread, time::Duration};
use tracing::{info, instrument, warn, Instrument};

#[instrument(level = "trace")]
pub async fn f() {
    let mut foo: u64 = 1;

    for i in 0..8 {
        log::trace!("Before my_great_span");

        async {
            thread::sleep(Duration::from_millis(3));
            tokio::time::sleep(Duration::from_millis(100)).await;
            foo += 1;
            info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
            log::trace!("Before my_other_span");
            async {
                thread::sleep(Duration::from_millis(2));
                tokio::time::sleep(Duration::from_millis(25)).await;
                warn!(yak_shaved = false, yak_count = -1, "failed to shave yak");
            }
            .instrument(tracing::trace_span!("my_other_span", foo = i % 2))
            .await;
        }
        .instrument(tracing::trace_span!(
            "my_great_span",
            foo = i % 2,
            bar = i % 4
        ))
        .await
    }
}

#[allow(unused)]
fn main() {}
