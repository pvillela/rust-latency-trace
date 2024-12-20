use dev_support::simple_span_counter::SimpleSpanCounter;
use latency_trace::{summary_stats, LatencyTrace};
use std::{thread, time::Duration};
use tracing::{info, instrument, level_filters::LevelFilter, trace, trace_span};
use tracing_subscriber::{prelude::*, Registry};

#[instrument(level = "info")]
fn f() {
    trace!("in f");
    for _ in 0..10 {
        trace_span!("loop_body").in_scope(|| {
            info!("in loop body");
            // Simulated work
            thread::sleep(Duration::from_micros(1200));

            g();
        });
    }
}

#[instrument(level = "info")]
fn g() {
    trace!("in g");
    // Simulated work
    thread::sleep(Duration::from_micros(800));
}

#[test]
fn test_layer_sync() {
    // LatencyTrace instance from which latency statistics will be extracted later.
    let lt = LatencyTrace::default();

    // Clone of the above that will be used as a `tracing_subscriber::layer::Layer` that can be
    // composed with other tracing layers. Add a filter so that only spans with level `INFO` or
    // higher priority (lower level) are aggregated.
    let ltl = lt.clone().with_filter(LevelFilter::INFO);

    // `SimpleSpanCounter` layer that can be composed with the above `LatencyTrace` layer.
    let spc = SimpleSpanCounter::new();

    // Clone of the above that will be used as a `tracing_subscriber::layer::Layer` that can be
    // composed with other tracing layers. Add a filter so that only spans with level `TRACE` or
    // higher priority (lower level) are aggregated.
    let spcl = spc.clone().with_filter(LevelFilter::TRACE);

    // Instantiate a layered subscriber and set it as the global default.
    let layered = Registry::default().with(ltl).with(spcl);
    layered.init();

    // Measure latencies.
    let latencies = lt.measure_latencies(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);

        // Test both layers count the same number of spans.
        let sp_name = span_group.name();
        let sp_count = stats.count;
        let simple_count = spc.get(sp_name);
        assert_eq!(sp_count, simple_count);
    }

    println!("\nSimpleCounts:");
    println!("{:?}", spc.dump());

    // Assert simple counts for "loop_body".
    assert_eq!(spc.get("loop_body"), spc.get("g"));
}
