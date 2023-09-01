use latency_trace::{
    group_by_all_fields, map::HashMapExt, measure_latencies_with_custom_grouping_tokio,
};
use std::collections::BTreeMap;
use tracing_core::callsite::Identifier;

mod common;
use common::{are_close, target_fn};

#[test]
fn test_grouping_by_all_fields() {
    let latencies = measure_latencies_with_custom_grouping_tokio(group_by_all_fields, target_fn);

    let span_group_count = 7;

    latencies.with(|info| {
        let parents = &info.parents;
        let timings = &info.timings;
        let name_to_callsite: BTreeMap<String, Identifier> = HashMapExt(&info.timings)
            .map_to_btree_map(|k, _| (k.name().to_owned(), k.callsite().clone()));

        assert_eq!(timings.len(), span_group_count, "Number of span groups");

        for (span_group, timing) in timings {
            let parent = parents.get(span_group.callsite()).unwrap().as_ref();
            let name = span_group.name();
            let props = Vec::from_iter(
                span_group
                    .props()
                    .iter()
                    .map(|p| (&p.0 as &str, &p.1 as &str)),
            );

            let total_time_mean = timing.total_time.mean();
            let total_time_count = timing.total_time.len();
            let active_time_mean = timing.active_time.mean();
            let active_time_count = timing.active_time.len();

            match name {
                "f" => {
                    let expected_parent = None;
                    let allowed_props: Vec<Vec<(&str, &str)>> = vec![vec![]];
                    let expected_total_time_mean = 130.0 * 8.0 * 1000.0;
                    let expected_active_time_mean = 5.0 * 8.0 * 1000.0;
                    let expected_total_time_count = 2;
                    let expected_active_time_count = 2;

                    assert_eq!(parent, expected_parent, "{name} parent");
                    assert!(allowed_props.contains(&props), "{name} props");

                    println!(
                        "** {name} total_time_mean: {total_time_mean}, {}",
                        expected_total_time_mean
                    );
                    assert!(
                        are_close(total_time_mean, expected_total_time_mean, 0.1),
                        "{name} total_time mean"
                    );

                    println!(
                        "** {name} total_time_count: {total_time_count}, {}",
                        expected_total_time_count
                    );
                    assert_eq!(
                        total_time_count, expected_total_time_count,
                        "{name} total_time count"
                    );

                    println!(
                        "** {name} active_time_mean: {active_time_mean}, {}",
                        expected_active_time_mean
                    );
                    assert!(
                        are_close(active_time_mean, expected_active_time_mean, 0.2),
                        "{name} active_time mean"
                    );

                    println!(
                        "** {name} active_time_count: {active_time_count}, {}",
                        expected_active_time_count
                    );
                    assert_eq!(
                        active_time_count, expected_active_time_count,
                        "{name} active_time count"
                    );
                }

                "my_great_span" => {
                    let expected_parent = Some(name_to_callsite.get("f").unwrap());
                    let allowed_props: Vec<Vec<(&str, &str)>> = vec![
                        vec![("bar", "0"), ("foo", "0")],
                        vec![("bar", "1"), ("foo", "1")],
                        vec![("bar", "2"), ("foo", "0")],
                        vec![("bar", "3"), ("foo", "1")],
                    ];
                    let expected_total_time_mean = 130.0 * 1000.0;
                    let expected_active_time_mean = 5.0 * 1000.0;
                    let expected_total_time_count = 4;
                    let expected_active_time_count = 4;
                }

                "my_other_span" => {
                    let expected_parent = Some(name_to_callsite.get("my_great_span").unwrap());
                    let allowed_props: Vec<Vec<(&str, &str)>> =
                        vec![vec![("foo", "0")], vec![("foo", "1")]];
                    let expected_total_time_mean = 27.0 * 1000.0;
                    let expected_active_time_mean = 2.0 * 1000.0;
                    let expected_total_time_count = 8;
                    let expected_active_time_count = 8;
                }

                _ => {
                    assert!(false, "*** unreachable code ***")
                }
            }
        }
    });
}
