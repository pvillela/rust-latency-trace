//! Functions to demonstrate the overhead associated with tracing and the framework.
//!
//! The nested spans with no other significant executable code, other than the loop and function call,
//! provide visibility to the overhead of span creation and processing.

use std::{hint::black_box, thread, time::Duration};
use tracing::{instrument, trace_span, Span};

#[instrument(level = "trace")]
pub fn deep_sync(nrepeats: usize, ntasks: usize) {
    #[instrument(level = "trace")]
    fn g(x: usize) -> usize {
        trace_span!("g-1").in_scope(|| {
            trace_span!("g-2").in_scope(|| trace_span!("g-3").in_scope(|| black_box(x)))
        })
    }

    let f = move || {
        trace_span!("f-1").in_scope(|| {
            trace_span!("f-2").in_scope(|| {
                trace_span!("f-3").in_scope(|| {
                    for i in 0..nrepeats {
                        trace_span!("loop_body+3").in_scope(|| {
                            trace_span!("loop_body+2").in_scope(|| {
                                trace_span!("loop_body+1").in_scope(|| {
                                    trace_span!("loop_body").in_scope(|| {
                                        thread::sleep(Duration::from_micros(0));
                                        trace_span!("empty").in_scope(|| {
                                            // Empty span used to show some of the tracing overhead.
                                            black_box(i);
                                        });

                                        black_box(g(i));
                                    });
                                });
                            });
                        });
                    }
                });
            });
        });
    };

    let current_span = Span::current();

    let hs = (0..ntasks)
        .map(|_| {
            let parent_span = current_span.clone();
            thread::spawn(move || {
                let _enter = parent_span.enter();
                f()
            })
        })
        .collect::<Vec<_>>();

    f();

    hs.into_iter().for_each(|h| h.join().unwrap());
}

pub fn deep_sync_un(nrepeats: usize, ntasks: usize) {
    fn g(x: usize) -> usize {
        black_box(x)
    }

    let f = move || {
        for i in 0..nrepeats {
            black_box(i);

            black_box(g(i));
        }
    };

    let hs = (0..ntasks).map(|_| thread::spawn(f)).collect::<Vec<_>>();
    f();
    hs.into_iter().for_each(|h| h.join().unwrap());
}
