//! Example of latency measurement for a simple sync function that does nothing,
//! to demonstrate the overhead associated with tracing and the framework.
//!
//! The nested spans with no other significant executable code, other than the loop and function call,
//! provide visibility to the overhead of span creation and processing, which is ~0.5-1 microseconds
//! per span instance on my 2022 Dell Inspiron 16.

use latency_trace::{summary_stats, BTreeMapExt, LatencyTrace};
use std::{hint::black_box, time::Instant};
use tracing::{instrument, trace_span};

#[instrument(level = "trace")]
fn f() {
    trace_span!("f-1").in_scope(|| {
        trace_span!("f-2").in_scope(|| {
            trace_span!("f-3").in_scope(|| {
                for i in 0..1000 {
                    trace_span!("loop_body+3").in_scope(|| {
                        trace_span!("loop_body+2").in_scope(|| {
                            trace_span!("loop_body+1").in_scope(|| {
                                trace_span!("loop_body").in_scope(|| {
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
}

#[instrument(level = "trace")]
fn g(x: i32) -> i32 {
    trace_span!("g-1")
        .in_scope(|| trace_span!("g-2").in_scope(|| trace_span!("g-3").in_scope(|| black_box(x))))
}

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let start = Instant::now();

    let latencies = LatencyTrace::default().measure_latencies(f);

    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}
