use latency_trace::{map::HashMapExt, Latencies, SpanGroup};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    future::Future,
    thread,
    time::Duration,
};
use tracing::{info, instrument, warn, Instrument};
use tracing_core::{callsite::Identifier, span::Attributes};

#[instrument(level = "trace")]
async fn f0() {
    let mut foo: u64 = 1;

    for i in 0..8 {
        log::trace!("Before my_great_span");

        async {
            thread::sleep(Duration::from_millis(3));
            tokio::time::sleep(Duration::from_millis(100)).await;
            foo += 1;
            info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
            log::trace!("Before my_other_span");
            async {
                thread::sleep(Duration::from_millis(2));
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
    let h1 = tokio::spawn(f0());
    let h2 = tokio::spawn(f0());
    _ = h1.await;
    _ = h2.await;
}

pub fn are_close(left: f64, right: f64, pct: f64) -> bool {
    let avg_abs = (left.abs() + right.abs()) / 2.0;
    (left - right).abs() <= avg_abs * pct
}

pub struct SpanNameTestSpec {
    pub expected_parent_name: Option<String>,
    pub allowed_props: Vec<Vec<(&'static str, &'static str)>>,
    pub expected_total_time_mean: f64,
    pub expected_active_time_mean: f64,
    pub expected_total_time_count: u64,
    pub expected_active_time_count: u64,
}

pub struct OverallTestSpec<F, G, H, Fut>
where
    F: Fn(G, H) -> Latencies,
    G: Fn(&Attributes) -> SpanGroup,
    H: Fn() -> Fut,
    Fut: Future<Output = ()>,
{
    pub measurement_fn: F,
    pub grouper: G,
    pub span_group_count: usize,
    pub target_fn: H,
    pub span_name_test_specs: HashMap<String, SpanNameTestSpec>,
}

pub fn run_test<F, G, H, Fut>(overall_test_def: OverallTestSpec<F, G, H, Fut>)
where
    F: Fn(G, H) -> Latencies,
    G: Fn(&Attributes) -> SpanGroup,
    H: Fn() -> Fut,
    Fut: Future<Output = ()>,
{
    let OverallTestSpec {
        measurement_fn,
        grouper,
        span_group_count,
        target_fn,
        span_name_test_specs,
    } = overall_test_def;

    let latencies = measurement_fn(grouper, target_fn);

    latencies.with(|info| {
        let parents = &info.parents;
        let timings = &info.timings;
        let name_to_callsite: BTreeMap<String, Identifier> = HashMapExt(&info.timings)
            .map_to_btree_map(|k, _| (k.name().to_owned(), k.callsite().clone()));

        assert_eq!(timings.len(), span_group_count, "Number of span groups");

        let mut names = HashSet::<&str>::new();

        for (span_group, timing) in timings {
            let parent = parents.get(span_group.callsite()).unwrap().as_ref();

            let name = span_group.name();
            names.insert(name);

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

            let run_test_for_span_name = |name: &str| {
                let SpanNameTestSpec {
                    expected_parent_name,
                    allowed_props,
                    expected_total_time_mean,
                    expected_total_time_count,
                    expected_active_time_mean,
                    expected_active_time_count,
                } = span_name_test_specs.get(name).unwrap();

                let expected_parent = expected_parent_name.clone().map(|name| {
                    parents
                        .get(name_to_callsite.get(&name).unwrap())
                        .unwrap()
                        .as_ref()
                        .unwrap()
                });
                assert_eq!(parent, expected_parent, "{name} parent");

                assert!(allowed_props.contains(&props), "{name} props");

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
                    are_close(active_time_mean, *expected_active_time_mean, 0.2),
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

            run_test_for_span_name(name);
        }

        let expected_names = HashSet::from(["f", "my_great_span", "my_other_span"]);
        assert_eq!(names, expected_names, "expected names");
    });
}
