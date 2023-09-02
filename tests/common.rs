use latency_trace::{
    map::{BTreeMapExt, HashMapExt},
    Latencies,
};
use std::{
    collections::{BTreeMap, HashSet},
    thread,
    time::Duration,
};
use tracing::{info, instrument, warn, Instrument};
use tracing_core::callsite::Identifier;

#[instrument(level = "trace")]
async fn f() {
    let mut foo: u64 = 1;

    for i in 0..8 {
        log::trace!("Before my_great_span");

        async {
            thread::sleep(Duration::from_millis(13));
            tokio::time::sleep(Duration::from_millis(100)).await;
            foo += 1;
            info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
            log::trace!("Before my_other_span");
            async {
                thread::sleep(Duration::from_millis(12));
                tokio::time::sleep(Duration::from_millis(25)).await;
                warn!(yak_shaved = false, yak_count = -1, "failed to shave yak");
            }
            .instrument(tracing::trace_span!("my_other_span", foo = i % 2))
            .await;
        }
        .instrument(tracing::trace_span!(
            "my_great_span",
            foo = i % 2,
            bar = i % 4
        ))
        .await
    }
}

pub async fn target_fn() {
    let h1 = tokio::spawn(f());
    let h2 = tokio::spawn(f());
    _ = h1.await;
    _ = h2.await;
}

pub fn are_close(left: f64, right: f64, pct: f64) -> bool {
    let avg_abs = (left.abs() + right.abs()) / 2.0;
    (left - right).abs() <= avg_abs * pct
}

#[derive(Debug)]
pub struct SpanNameTestSpec {
    pub expected_parent_name: Option<&'static str>,
    pub expected_props: Vec<Vec<(&'static str, &'static str)>>,
    pub expected_total_time_mean: f64,
    pub expected_active_time_mean: f64,
    pub expected_total_time_count: u64,
    pub expected_active_time_count: u64,
}

pub struct TestSpec {
    pub span_group_count: usize,
    pub span_name_test_specs: BTreeMap<&'static str, SpanNameTestSpec>,
}

pub fn run_test(latencies: &Latencies, test_spec: &TestSpec) {
    let TestSpec {
        span_group_count,
        span_name_test_specs,
    } = test_spec;

    let expected_names: HashSet<&'static str> = span_name_test_specs.keys().map(|s| *s).collect();
    let mut remaining_names = expected_names.clone();

    let mut remaining_props =
        BTreeMapExt(&span_name_test_specs).map_values(|v| v.expected_props.clone());

    latencies.with(|info| {
        let parents = &info.parents;
        let timings = &info.timings;
        let name_to_callsite: BTreeMap<String, Identifier> = HashMapExt(&info.timings)
            .map_to_btree_map(|k, _| (k.name().to_owned(), k.callsite().clone()));

        assert_eq!(timings.len(), *span_group_count, "Number of span groups");

        // Force tests to proceed aphabetically by span name.
        for (name, spec) in span_name_test_specs {
            let name = *name;
            assert!(
                expected_names.contains(name),
                "{name} must be in expected_names"
            );
            remaining_names.remove(name);

            let SpanNameTestSpec {
                expected_parent_name,
                expected_props,
                expected_total_time_mean,
                expected_total_time_count,
                expected_active_time_mean,
                expected_active_time_count,
            } = spec;

            let expected_parent = expected_parent_name
                .clone()
                .map(|parent_name| name_to_callsite.get(parent_name).unwrap());

            for (span_group, timing) in timings.iter().filter(|(k, _)| k.name() == name) {
                let parent = parents.get(span_group.callsite()).unwrap().as_ref();

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

                {
                    assert_eq!(parent, expected_parent, "{name} parent");

                    assert!(
                        expected_props.contains(&props),
                        "{name} props invalid: props={:?}, expected_props={:?}",
                        props,
                        expected_props
                    );

                    // Remove props from remaining_props. For each name, an allowed props value should occur exactly once.
                    {
                        let v = remaining_props.get_mut(name).unwrap();
                        let idx = v
                            .iter()
                            .position(|p| *p == props)
                            .expect(&format!("props={:?} not found for {name}", props));
                        v.remove(idx);
                    }

                    println!(
                        "** {name} total_time_mean: {total_time_mean}, {}",
                        expected_total_time_mean
                    );
                    assert!(
                        are_close(total_time_mean, *expected_total_time_mean, 0.1),
                        "{name} total_time mean"
                    );

                    println!(
                        "** {name} total_time_count: {total_time_count}, {}",
                        expected_total_time_count
                    );
                    assert_eq!(
                        total_time_count, *expected_total_time_count,
                        "{name} total_time count"
                    );

                    println!(
                        "** {name} active_time_mean: {active_time_mean}, {}",
                        expected_active_time_mean
                    );
                    assert!(
                        are_close(active_time_mean, *expected_active_time_mean, 0.1),
                        "{name} active_time mean"
                    );

                    println!(
                        "** {name} active_time_count: {active_time_count}, {}",
                        expected_active_time_count
                    );
                    assert_eq!(
                        active_time_count, *expected_active_time_count,
                        "{name} active_time count"
                    );
                };
            }
        }
        assert!(
            remaining_names.is_empty(),
            "remaining_names must be empty at the end"
        );
        assert!(
            remaining_props.iter().all(|(_, v)| v.is_empty()),
            "remaining_props must be empty for each name at the end"
        );
    });
}
