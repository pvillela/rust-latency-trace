mod common;

use common::{run_test, SpanNameTestSpec, TestSpec, E};
use dev_utils::target_fns::target_fn;
use latency_trace::measure_latencies_tokio;
use std::collections::BTreeMap;

#[test]
fn test_default_grouping() {
    let latencies = measure_latencies_tokio(target_fn);

    let test_spec = TestSpec {
        span_group_count: 12,
        span_name_test_specs: BTreeMap::from([
            (
                "root_async_1",
                SpanNameTestSpec {
                    expected_props: vec![E],
                    expected_parent_names: vec![],
                    expected_parent_props: vec![],
                    expected_total_time_mean: 150.0 * 8.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 8.0 * 1000.0,
                    expected_total_time_count: 1,
                    expected_active_time_count: 1,
                    expected_agg_by_name_count: 1,
                },
            ),
            (
                "root_async_2",
                SpanNameTestSpec {
                    expected_props: vec![E],
                    expected_parent_names: vec![],
                    expected_parent_props: vec![],
                    expected_total_time_mean: 150.0 * 8.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 8.0 * 1000.0,
                    expected_total_time_count: 1,
                    expected_active_time_count: 1,
                    expected_agg_by_name_count: 1,
                },
            ),
            (
                "f",
                SpanNameTestSpec {
                    expected_props: vec![E],
                    expected_parent_names: vec!["root_async_1", "root_async_2"],
                    expected_parent_props: vec![E],
                    expected_total_time_mean: 150.0 * 8.0 * 1000.0,
                    expected_active_time_mean: 25.0 * 8.0 * 1000.0,
                    expected_total_time_count: 1,
                    expected_active_time_count: 1,
                    expected_agg_by_name_count: 2,
                },
            ),
            (
                "outer_async_span",
                SpanNameTestSpec {
                    expected_props: vec![E],
                    expected_parent_names: vec!["f"],
                    expected_parent_props: vec![E],
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
                    expected_props: vec![E],
                    expected_parent_names: vec!["outer_async_span"],
                    expected_parent_props: vec![E],
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
                    expected_props: vec![E],
                    expected_parent_names: vec!["outer_async_span"],
                    expected_parent_props: vec![E],
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
                    expected_props: vec![E],
                    expected_parent_names: vec!["inner_async_span"],
                    expected_parent_props: vec![E],
                    expected_total_time_mean: 12.0 * 1000.0,
                    expected_active_time_mean: 12.0 * 1000.0,
                    expected_total_time_count: 8,
                    expected_active_time_count: 8,
                    expected_agg_by_name_count: 16,
                },
            ),
        ]),
    };

    run_test(&latencies, test_spec);
}
