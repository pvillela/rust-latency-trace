mod common;

use common::run_test;
use dev_utils::{
    target_fns::target_fn,
    test_support::{
        span_name_test_spec_f, span_name_test_spec_inner_async_span,
        span_name_test_spec_outer_async_span, span_name_test_spec_root_async_1,
        span_name_test_spec_root_async_2, span_name_test_spec_sync_span_1,
        span_name_test_spec_sync_span_2, TestSpec, E,
    },
};
use latency_trace::{group_by_given_fields, LatencyTrace};
use std::collections::BTreeMap;

#[test]
#[allow(clippy::identity_op)]
fn test_grouping_by_given_fields() {
    let latencies = LatencyTrace::default()
        .with_span_grouper(group_by_given_fields(&["foo"]))
        .measure_latencies_tokio(target_fn);

    // Number of span groups by name
    let n_root_async_1: u64 = 1;
    let n_root_async_2: u64 = 1;
    let n_f: u64 = (n_root_async_1 + n_root_async_2) * 1;
    let n_outer_async_span: u64 = n_f * 2;
    let n_inner_async_span: u64 = n_outer_async_span * 1;
    let n_sync_span_1: u64 = n_outer_async_span * 1;
    let n_sync_span_2: u64 = n_inner_async_span * 1;

    let test_spec = TestSpec {
        spec_name: "grouping_by_given_fields",

        span_group_count: (n_root_async_1
            + n_root_async_2
            + n_f
            + n_outer_async_span
            + n_inner_async_span
            + n_sync_span_1
            + n_sync_span_2) as usize,

        span_name_test_specs: BTreeMap::from([
            (
                "root_async_1",
                span_name_test_spec_root_async_1(vec![vec![("foo", "1")]], vec![], n_root_async_1),
            ),
            (
                "root_async_2",
                span_name_test_spec_root_async_2(vec![E], vec![], n_root_async_2),
            ),
            (
                "f",
                span_name_test_spec_f(vec![E], vec![vec![("foo", "1")], E], n_f),
            ),
            (
                "outer_async_span",
                span_name_test_spec_outer_async_span(
                    vec![vec![("foo", "0")], vec![("foo", "1")]],
                    vec![E],
                    n_outer_async_span,
                ),
            ),
            (
                "inner_async_span",
                span_name_test_spec_inner_async_span(
                    vec![vec![("foo", "0")], vec![("foo", "1")]],
                    vec![vec![("foo", "0")], vec![("foo", "1")]],
                    n_inner_async_span,
                ),
            ),
            (
                "sync_span_1",
                span_name_test_spec_sync_span_1(
                    vec![E],
                    vec![vec![("foo", "0")], vec![("foo", "1")]],
                    n_sync_span_1,
                ),
            ),
            (
                "sync_span_2",
                span_name_test_spec_sync_span_2(
                    vec![E],
                    vec![vec![("foo", "0")], vec![("foo", "1")]],
                    n_sync_span_2,
                ),
            ),
        ]),
    };

    run_test(&latencies, test_spec);
}
