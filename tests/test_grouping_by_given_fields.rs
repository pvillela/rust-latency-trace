mod common;

use common::{run_test, target_fn, SpanNameTestSpec, TestSpec};
use latency_trace::{group_by_given_fields, measure_latencies_with_custom_grouping_tokio};
use std::collections::BTreeMap;

#[test]
fn test_grouping_by_given_fields() {
    let latencies =
        measure_latencies_with_custom_grouping_tokio(group_by_given_fields(&["foo"]), target_fn);

    let test_spec = TestSpec {
        span_group_count: 5,
        span_name_test_specs: BTreeMap::from([
            (
                "f",
                SpanNameTestSpec {
                    expected_parent_name: None,
                    expected_props: vec![vec![]],
                    expected_total_time_mean: 130.0 * 8.0 * 1000.0,
                    expected_active_time_mean: 5.0 * 8.0 * 1000.0,
                    expected_total_time_count: 2,
                    expected_active_time_count: 2,
                },
            ),
            (
                "my_great_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("f"),
                    expected_props: vec![vec![("foo", "0")], vec![("foo", "1")]],
                    expected_total_time_mean: 130.0 * 1000.0,
                    expected_active_time_mean: 5.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                },
            ),
            (
                "my_other_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("my_great_span"),
                    expected_props: vec![vec![("foo", "0")], vec![("foo", "1")]],
                    expected_total_time_mean: 27.0 * 1000.0,
                    expected_active_time_mean: 2.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                },
            ),
        ]),
    };

    run_test(&latencies, &test_spec);
}
