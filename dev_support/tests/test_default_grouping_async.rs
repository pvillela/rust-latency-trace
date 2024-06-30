use dev_support::{
    elab_fns::elab_async,
    test_support::{
        run_test, span_name_test_spec_f, span_name_test_spec_inner_span,
        span_name_test_spec_outer_span, span_name_test_spec_root_1, span_name_test_spec_root_2,
        span_name_test_spec_span_1, span_name_test_spec_span_2, TestSpec, E,
    },
};
use latency_trace::LatencyTraceOld;
use std::collections::BTreeMap;

#[test]
#[allow(clippy::identity_op)]
fn test_default_grouping() {
    let latencies = LatencyTraceOld::default()
        .measure_latencies_tokio(elab_async)
        .unwrap();

    // Number of span groups by name
    let n_root_1: u64 = 1;
    let n_root_2: u64 = 1;
    let n_f: u64 = (n_root_1 + n_root_2) * 1;
    let n_outer_span: u64 = n_f * 1;
    let n_inner_span: u64 = n_outer_span * 1;
    let n_span_1: u64 = n_outer_span * 1;
    let n_span_2: u64 = n_inner_span * 1;

    let test_spec = TestSpec {
        spec_name: "default_grouping",

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
