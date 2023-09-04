mod common;

use common::{run_test, target_fn, SpanNameTestSpec, TestSpec};
use latency_trace::{group_by_all_fields, measure_latencies_with_custom_grouping_tokio};
use std::collections::BTreeMap;

#[test]
fn test_grouping_by_all_fields() {
    let latencies = measure_latencies_with_custom_grouping_tokio(group_by_all_fields, target_fn);

    let test_spec = TestSpec {
        span_group_count: 17,
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
                },
            ),
            (
                "outer_async_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("f"),
                    expected_props: vec![
                        vec![vec![("bar", "0"), ("foo", "0")], vec![]],
                        vec![vec![("bar", "1"), ("foo", "1")], vec![]],
                        vec![vec![("bar", "2"), ("foo", "0")], vec![]],
                        vec![vec![("bar", "3"), ("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 150.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 1000.0,
                    expected_total_time_count: 4,
                    expected_active_time_count: 4,
                },
            ),
            (
                "inner_async_span",
                SpanNameTestSpec {
                    expected_parent_name: Some("outer_async_span"),
                    expected_props: vec![
                        vec![vec![("foo", "0")], vec![("bar", "0"), ("foo", "0")], vec![]],
                        vec![vec![("foo", "1")], vec![("bar", "1"), ("foo", "1")], vec![]],
                        vec![vec![("foo", "0")], vec![("bar", "2"), ("foo", "0")], vec![]],
                        vec![vec![("foo", "1")], vec![("bar", "3"), ("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 37.0 * 1000.0,
                    expected_active_time_mean: 12.0 * 1000.0,
                    expected_total_time_count: 4,
                    expected_active_time_count: 4,
                },
            ),
            (
                "sync_span_1",
                SpanNameTestSpec {
                    expected_parent_name: Some("outer_async_span"),
                    expected_props: vec![
                        vec![vec![], vec![("bar", "0"), ("foo", "0")], vec![]],
                        vec![vec![], vec![("bar", "1"), ("foo", "1")], vec![]],
                        vec![vec![], vec![("bar", "2"), ("foo", "0")], vec![]],
                        vec![vec![], vec![("bar", "3"), ("foo", "1")], vec![]],
                    ],
                    expected_total_time_mean: 13.0 * 1000.0,
                    expected_active_time_mean: 13.0 * 1000.0,
                    expected_total_time_count: 4,
                    expected_active_time_count: 4,
                },
            ),
            (
                "sync_span_2",
                SpanNameTestSpec {
                    expected_parent_name: Some("inner_async_span"),
                    expected_props: vec![
                        vec![
                            vec![],
                            vec![("foo", "0")],
                            vec![("bar", "0"), ("foo", "0")],
                            vec![],
                        ],
                        vec![
                            vec![],
                            vec![("foo", "1")],
                            vec![("bar", "1"), ("foo", "1")],
                            vec![],
                        ],
                        vec![
                            vec![],
                            vec![("foo", "0")],
                            vec![("bar", "2"), ("foo", "0")],
                            vec![],
                        ],
                        vec![
                            vec![],
                            vec![("foo", "1")],
                            vec![("bar", "3"), ("foo", "1")],
                            vec![],
                        ],
                    ],
                    expected_total_time_mean: 12.0 * 1000.0,
                    expected_active_time_mean: 12.0 * 1000.0,
                    expected_total_time_count: 4,
                    expected_active_time_count: 4,
                },
            ),
        ]),
    };

    run_test(&latencies, &test_spec);
}
