use crate::test_support::{SpanNameTestSpec, TestSpec};
use latency_trace::{SpanGroup, Timings};
use std::collections::HashSet;

fn f64_are_close(left: f64, right: f64, pct: f64) -> bool {
    let avg_abs = (left.abs() + right.abs()) / 2.0;
    (left - right).abs() <= avg_abs * pct
}

pub fn run_test(tmgs: &Timings, test_spec: TestSpec) {
    let TestSpec {
        spec_name,
        span_group_count,
        span_name_test_specs,
    } = test_spec;

    let expected_name_set: HashSet<&'static str> = span_name_test_specs.keys().copied().collect();
    let mut name_set: HashSet<&'static str> = HashSet::new();

    assert_eq!(
        tmgs.len(),
        span_group_count,
        "spec_name={spec_name}: Number of span groups - tmgs.keys()={:?}",
        tmgs.keys()
    );

    let aggregator = |sg: &SpanGroup| sg.name();

    let consistent_timings = tmgs.aggregator_is_consistent(aggregator);
    assert!(consistent_timings, "consistent_timings");

    let agg_timings = tmgs.aggregate(aggregator);
    assert_eq!(
        agg_timings.len(),
        span_name_test_specs.len(),
        "spec_name={spec_name}: aggregation by name - number of aggregate values"
    );

    let parents = tmgs.span_group_to_parent();

    // Force tests to proceed aphabetically by span name.
    for (name, spec) in span_name_test_specs {
        assert!(
            expected_name_set.contains(name),
            "spec_name={spec_name}: name={name} must be in expected_names={expected_name_set:?}"
        );
        name_set.insert(name);

        let SpanNameTestSpec {
            expected_props,
            expected_parent_names,
            expected_parent_props,
            expected_mean: expected_timing_mean,
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
            expected_parent_names.iter().copied().collect();
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

            let timing_mean = agg_timing.mean();
            let timing_count = agg_timing.len();

            assert!(
                f64_are_close(timing_mean, expected_timing_mean, 0.2),
                "spec_name={spec_name}: {name} aggregate timing_mean: actual={timing_mean}, expected={expected_timing_mean}"
            );

            assert_eq!(
                timing_count, expected_agg_by_name_count,
                "spec_name={spec_name}: {name} aggregate timing_count"
            );
        }

        // Assertions by SpanGroup
        for (span_group, timing) in tmgs.iter().filter(|(k, _)| k.name() == name) {
            let props = span_group.props();
            props_set.insert(props.into());

            let parent = parents.get(span_group).unwrap();
            let parent_name = parent.as_ref().map(|p| p.name());
            let parent_props = parent.iter().map(|p| Vec::from(p.props())).next();

            // Insert parent_name and parent_props into corresponding sets
            parent_name.map(|parent_name| parent_name_set.insert(parent_name));
            parent_props.map(|parent_props| parent_props_set.insert(parent_props));

            let timing_mean = timing.mean();
            let timing_count = timing.len();

            {
                assert!(
                    f64_are_close(timing_mean, expected_timing_mean, 0.25),
                    "spec_name={spec_name}: {name} timing_mean: actual={timing_mean}, expected={expected_timing_mean}"
                );

                assert_eq!(
                    timing_count, expected_timing_count,
                    "spec_name={spec_name}: {name} timing_count"
                );
            };
        }

        assert_eq!(props_set, expected_props_set, "{name} props_set");
        assert_eq!(
            parent_name_set, expected_parent_name_set,
            "spec_name={spec_name}: {name} parent_name_set"
        );
        assert_eq!(
            parent_props_set, expected_parent_props_set,
            "spec_name={spec_name}: {name} parent_props_set"
        );
    }

    assert_eq!(
        name_set, expected_name_set,
        "spec_name={spec_name}: name_set"
    );
}
