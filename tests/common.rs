use std::{ops::Deref, thread, time::Duration};
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

pub fn are_close(left: f64, right: f64, pct: f64) -> bool {
    let avg_abs = (left.abs() + right.abs()) / 2.0;
    (left - right).abs() <= avg_abs * pct
}

// Key to access Info.timings.
pub struct Key<'a> {
    pub name: &'a str,
    pub props: &'a [(&'a str, &'a str)],
}

impl Key<'_> {
    pub fn name(&self) -> &str {
        self.name
    }

    pub fn props(&self) -> Vec<(String, String)> {
        Vec::from_iter(self.props.iter().map(|p| (p.0.to_owned(), p.1.to_owned())))
    }
}
