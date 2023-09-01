use latency_trace::{map::HashMapExt, measure_latencies_tokio, Timing};
use std::collections::BTreeMap;
use tracing_core::callsite::Identifier;

mod common;
use common::{are_close, target_fn};

#[test]
fn test_default_grouping() {
    let latencies = measure_latencies_tokio(target_fn);

    latencies.with(|info| {
        let parents = &info.parents;

        let name_to_timing: BTreeMap<String, Timing> =
            HashMapExt(&info.timings).map_to_btree_map(|k, v| (k.name().to_owned(), v.clone()));

        let name_to_callsite: BTreeMap<String, Identifier> = HashMapExt(&info.timings)
            .map_to_btree_map(|k, _| (k.name().to_owned(), k.callsite().clone()));

        for name in ["f", "my_great_span", "my_other_span"] {
            let parent = parents
                .get(name_to_callsite.get(name).unwrap())
                .unwrap()
                .as_ref();
            let timing = name_to_timing.get(name).unwrap();
            let total_time_mean = timing.total_time.mean();
            let total_time_count = timing.total_time.len();
            let active_time_mean = timing.active_time.mean();
            let active_time_count = timing.active_time.len();

            match name {
                "f" => {
                    let expected_parent = None;
                    let expected_total_time_mean = 130.0 * 8.0 * 1000.0;
                    let expected_active_time_mean = 5.0 * 8.0 * 1000.0;
                    let expected_total_time_count = 2;
                    let expected_active_time_count = 2;

                    assert_eq!(parent, expected_parent, "{name} parent");

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
                    let expected_total_time_mean = 130.0 * 1000.0;
                    let expected_active_time_mean = 5.0 * 1000.0;
                    let expected_total_time_count = 16;
                    let expected_active_time_count = 16;

                    assert_eq!(parent, expected_parent, "{name} parent");

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

                "my_other_span" => {
                    let expected_parent = Some(name_to_callsite.get("my_great_span").unwrap());
                    let expected_total_time_mean = 27.0 * 1000.0;
                    let expected_active_time_mean = 2.0 * 1000.0;
                    let expected_total_time_count = 16;
                    let expected_active_time_count = 16;

                    assert_eq!(parent, expected_parent, "{name} parent");

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

                _ => {}
            }
        }
    });
}
