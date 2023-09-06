mod common;

use common::{run_test, SpanNameTestSpec, TestSpec};
use dev_utils::target_fns::target_fn;
use latency_trace::{group_by_given_fields, measure_latencies_with_custom_grouping_tokio};
use std::collections::BTreeMap;

#[test]
fn test_grouping_by_given_fields() {
    let latencies =
        measure_latencies_with_custom_grouping_tokio(group_by_given_fields(&["foo"]), target_fn);

    let test_spec = TestSpec {
        span_group_count: 9,
        span_name_test_specs: BTreeMap::from([
            (
                "f",
                SpanNameTestSpec {
                    expected_parent_name: None,
                    expected_props: vec![vec![vec![]]],
                    expected_total_time_mean: 150.0 * 8.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 8.0 * 1000.0,
                    expected_total_time_count: 2,
                    expected_active_time_count: 2,
                    expected_agg_by_name_count: 2,
                },
            ),
            (
                "outer_async_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("f"),
                    expected_props: vec![
                        vec![vec![("foo", "0")], vec![]],
                        vec![vec![("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 150.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                    expected_agg_by_name_count: 16,
                },
            ),
            (
                "inner_async_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("outer_async_span"),
                    expected_props: vec![
                        vec![vec![("foo", "0")], vec![("foo", "0")], vec![]],
                        vec![vec![("foo", "1")], vec![("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 37.0 * 1000.0,
                    expected_active_time_mean: 12.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                    expected_agg_by_name_count: 16,
                },
            ),
            (
                "sync_span_1",
                SpanNameTestSpec {
                    expected_parent_name: Some("outer_async_span"),
                    expected_props: vec![
                        vec![vec![], vec![("foo", "0")], vec![]],
                        vec![vec![], vec![("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 13.0 * 1000.0,
                    expected_active_time_mean: 13.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                    expected_agg_by_name_count: 16,
                },
            ),
            (
                "sync_span_2",
                SpanNameTestSpec {
                    expected_parent_name: Some("inner_async_span"),
                    expected_props: vec![
                        vec![vec![], vec![("foo", "0")], vec![("foo", "0")], vec![]],
                        vec![vec![], vec![("foo", "1")], vec![("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 12.0 * 1000.0,
                    expected_active_time_mean: 12.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                    expected_agg_by_name_count: 16,
                },
            ),
        ]),
    };

    run_test(&latencies, &test_spec);
}
