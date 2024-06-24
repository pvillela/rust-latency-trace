mod common;

use common::run_test;
use dev_utils::{
    elab_fns::{
        elab_sync_gated, PROBE_GATE_F1_PROBE_READY, PROBE_GATE_F2_PROBE_READY,
        PROBE_GATE_F_PROCEED,
    },
    gater::Gater,
    test_support::{
        span_name_test_spec_f, span_name_test_spec_inner_span, span_name_test_spec_outer_span,
        span_name_test_spec_root_1, span_name_test_spec_root_2, span_name_test_spec_span_1,
        span_name_test_spec_span_2, SpanNameTestSpec, TestSpec, E,
    },
};
use latency_trace::LatencyTrace;
use std::{collections::BTreeMap, sync::Arc};

#[tokio::test]
#[allow(clippy::identity_op)]
async fn test_probed() {
    std::env::set_var("RUST_LOG", "latency_trace=trace,thread_local_collect=trace");
    // _ = env_logger::try_init();

    let probe_gater = Arc::new(Gater::new("probe_gater"));

    let probed = LatencyTrace::default().measure_latencies_probed({
        let probe_gater = probe_gater.clone();
        || elab_sync_gated(Some(probe_gater))
    });

    // Number of span groups by name
    let n_root_1: u64 = 1;
    let n_root_2: u64 = 1;
    let n_f: u64 = (n_root_1 + n_root_2) * 1;
    let n_outer_span: u64 = n_f * 1;
    let n_inner_span: u64 = n_outer_span * 1;
    let n_span_1: u64 = n_outer_span * 1;
    let n_span_2: u64 = n_inner_span * 1;

    // Test interim latencies
    {
        probe_gater.wait_for_async(PROBE_GATE_F1_PROBE_READY).await;
        probe_gater.wait_for_async(PROBE_GATE_F2_PROBE_READY).await;
        let latencies = probed.probe_latencies();
        probe_gater.open(PROBE_GATE_F_PROCEED);

        let test_spec_root_1 = SpanNameTestSpec {
            expected_mean: 0.0,
            expected_timing_count: 0,
            expected_agg_by_name_count: 0,
            ..span_name_test_spec_root_1(vec![E], vec![], n_root_1)
        };

        let test_spec_root_2 = SpanNameTestSpec {
            expected_mean: 0.0,
            expected_timing_count: 0,
            expected_agg_by_name_count: 0,
            ..span_name_test_spec_root_2(vec![E], vec![], n_root_2)
        };

        let test_spec_f = SpanNameTestSpec {
            expected_mean: 0.0,
            expected_timing_count: 0,
            expected_agg_by_name_count: 0,
            ..span_name_test_spec_f(vec![E], vec![E], n_f)
        };

        let test_spec_outer_span = SpanNameTestSpec {
            expected_timing_count: 4,
            expected_agg_by_name_count: 8,
            ..span_name_test_spec_outer_span(vec![E], vec![E], n_outer_span)
        };

        let test_spec_inner_span = SpanNameTestSpec {
            expected_timing_count: 4,
            expected_agg_by_name_count: 8,
            ..span_name_test_spec_inner_span(vec![E], vec![E], n_inner_span)
        };

        let test_spec_span_1 = SpanNameTestSpec {
            expected_timing_count: 4,
            expected_agg_by_name_count: 8,
            ..span_name_test_spec_span_1(vec![E], vec![E], n_span_1)
        };

        let test_spec_span_2 = SpanNameTestSpec {
            expected_timing_count: 4,
            expected_agg_by_name_count: 8,
            ..span_name_test_spec_span_2(vec![E], vec![E], n_span_2)
        };

        let test_spec = TestSpec {
            spec_name: "probed_interim",

            span_group_count: (n_root_1
                + n_root_2
                + n_f
                + n_outer_span
                + n_inner_span
                + n_span_1
                + n_span_2) as usize,

            span_name_test_specs: BTreeMap::from([
                ("root_1", test_spec_root_1),
                ("root_2", test_spec_root_2),
                ("f", test_spec_f),
                ("outer_span", test_spec_outer_span),
                ("inner_span", test_spec_inner_span),
                ("span_1", test_spec_span_1),
                ("span_2", test_spec_span_2),
            ]),
        };

        run_test(&latencies, test_spec);
    }

    // Test final latencies
    {
        let latencies = probed.wait_and_report();

        let test_spec = TestSpec {
            spec_name: "probed_final",
            span_group_count: (n_root_1
                + n_root_2
                + n_f
                + n_outer_span
                + n_inner_span
                + n_span_1
                + n_span_2) as usize,

            span_name_test_specs: BTreeMap::from([
                (
                    "root_1",
                    span_name_test_spec_root_1(vec![E], vec![], n_root_1),
                ),
                (
                    "root_2",
                    span_name_test_spec_root_2(vec![E], vec![], n_root_2),
                ),
                ("f", span_name_test_spec_f(vec![E], vec![E], n_f)),
                (
                    "outer_span",
                    span_name_test_spec_outer_span(vec![E], vec![E], n_outer_span),
                ),
                (
                    "inner_span",
                    span_name_test_spec_inner_span(vec![E], vec![E], n_inner_span),
                ),
                (
                    "span_1",
                    span_name_test_spec_span_1(vec![E], vec![E], n_span_1),
                ),
                (
                    "span_2",
                    span_name_test_spec_span_2(vec![E], vec![E], n_span_2),
                ),
            ]),
        };

        run_test(&latencies, test_spec);
    }
}
