use dev_utils::test_support::{f64_are_close, u64_comparator, SpanNameTestSpec, TestSpec};
use latency_trace::{Latencies, TimingsAggregate};
use std::collections::HashSet;

pub fn run_test(ltcs: &Latencies, test_spec: TestSpec) {
    run_test_general(ltcs, test_spec, u64_comparator(0.0))
}

pub fn run_test_general(
    ltcs: &Latencies,
    test_spec: TestSpec,
    timing_count_comparator: impl Fn(u64, u64) -> bool,
) {
    let span_groups_and_keys_are_same = ltcs
        .span_groups()
        .iter()
        .zip(ltcs.timings().keys())
        .all(|(left, right)| left == right);
    assert!(
        span_groups_and_keys_are_same,
        "span_groups_and_keys_are_same"
    );

    let TestSpec {
        span_group_count,
        span_name_test_specs,
    } = test_spec;

    let expected_name_set: HashSet<&'static str> =
        span_name_test_specs.keys().map(|s| *s).collect();
    let mut name_set: HashSet<&'static str> = HashSet::new();

    assert_eq!(
        ltcs.span_groups().len(),
        span_group_count,
        "Number of span groups"
    );

    let (agg_timings, consistent_timings) = ltcs.timings().aggregate(|sg| sg.name());
    assert!(consistent_timings, "consistent_timings");
    assert_eq!(
        agg_timings.len(),
        span_name_test_specs.len(),
        "aggregation by name - number of aggregate values"
    );

    // Force tests to proceed aphabetically by span name.
    for (name, spec) in span_name_test_specs {
        assert!(
            expected_name_set.contains(name),
            "{name} must be in expected_names"
        );
        name_set.insert(name);

        let SpanNameTestSpec {
            expected_props,
            expected_parent_names,
            expected_parent_props,
            expected_mean: expected_total_time_mean,
            expected_timing_count,
            expected_agg_by_name_count,
        } = spec;

        let expected_props_set: HashSet<Vec<(String, String)>> = expected_props
            .iter()
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect()
            })
            .collect();
        let mut props_set: HashSet<Vec<(String, String)>> = HashSet::new();

        let expected_parent_name_set: HashSet<&'static str> =
            expected_parent_names.iter().map(|name| *name).collect();
        let mut parent_name_set: HashSet<&'static str> = HashSet::new();

        let expected_parent_props_set: HashSet<Vec<(String, String)>> = expected_parent_props
            .iter()
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                    .collect()
            })
            .collect();
        let mut parent_props_set: HashSet<Vec<(String, String)>> = HashSet::new();

        // Aggregation assertions.
        {
            let agg_timing = agg_timings.get(name).unwrap();

            let total_time_mean = agg_timing.mean();
            let total_time_count = agg_timing.len();

            assert!(
                f64_are_close(total_time_mean, expected_total_time_mean, 0.2),
                "{name} aggregate total_time_mean: {total_time_mean}, {}",
                expected_total_time_mean
            );

            assert!(
                timing_count_comparator(total_time_count, expected_agg_by_name_count),
                "{name} aggregate total_time_count: {total_time_count}, {expected_agg_by_name_count}"
            );
        }

        // Assertions by SpanGroup
        for (span_group, timing) in ltcs.timings().iter().filter(|(k, _)| k.name() == name) {
            let idx = span_group.idx();
            assert_eq!(
                span_group,
                ltcs.span_groups().get(idx).unwrap(),
                "the span_group must be found in span_groups vector at position `idx`: {:?}",
                span_group
            );

            let props = span_group.props();
            props_set.insert(props.into());

            let parent_idx = span_group.parent_idx();
            parent_idx.map(|parent_idx| {
                assert!(
                    parent_idx < idx,
                    "parent_idx {parent_idx} must be less than span_group.idx {idx}; name={name}",
                );
            });

            let parent = span_group
                .parent_idx()
                .map(|parent_idx| ltcs.span_groups()[parent_idx].clone());
            let parent_name = parent.as_ref().map(|p| p.name());
            let parent_props: Option<Vec<(String, String)>> = parent.map(|p| p.props().into());

            // Insert parent_name and parent_props into corresponding sets
            parent_name.map(|parent_name| parent_name_set.insert(parent_name));
            parent_props.map(|parent_props| parent_props_set.insert(parent_props));

            let total_time_mean = timing.mean();
            let total_time_count = timing.len();

            {
                assert!(
                    f64_are_close(total_time_mean, expected_total_time_mean, 0.25),
                    "{name} total_time_mean: {total_time_mean}, {expected_total_time_mean}"
                );

                assert!(
                    timing_count_comparator(total_time_count, expected_timing_count),
                    "{name} total_time_count: {total_time_count}, {expected_timing_count}"
                );
            };
        }

        assert_eq!(props_set, expected_props_set, "{name} props_set");
        assert_eq!(
            parent_name_set, expected_parent_name_set,
            "{name} parent_name_set"
        );
        assert_eq!(
            parent_props_set, expected_parent_props_set,
            "{name} parent_props_set"
        );
    }

    assert_eq!(name_set, expected_name_set, "name_set");
}
