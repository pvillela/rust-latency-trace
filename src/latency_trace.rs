//! supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented wth the Rust [tracing](https://crates.io/crates/tracing) library, this library
//! uses the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture both total and active
//! span timings, where:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use hdrhistogram::Histogram;
use log;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use thread_local_drop::{self, Control, Holder};
use tracing::{callsite::Identifier, Id, Subscriber};
use tracing_core::span::Attributes;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

//=================
// SpanGroup

pub type Props = Vec<Vec<(&'static str, String)>>;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) callsite: Identifier,
    pub(crate) code_line: String,
    pub(crate) name: &'static str,
    pub(crate) props: Props,
}

impl SpanGroup {
    pub fn new(attrs: &Attributes, props: Props) -> Self {
        let meta = attrs.metadata();
        SpanGroup {
            callsite: meta.callsite(),
            code_line: format!("{}:{}", meta.module_path().unwrap(), meta.line().unwrap()),
            name: meta.name(),
            props,
        }
    }

    pub fn callsite(&self) -> &Identifier {
        &self.callsite
    }

    pub fn code_line(&self) -> &str {
        &self.code_line
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn props(&self) -> &Props {
        &self.props
    }
}

//=================
// Timing

#[derive(Clone, Debug)]
pub struct Timing {
    pub total_time: Histogram<u64>,
    pub active_time: Histogram<u64>,
}

impl Timing {
    fn new() -> Self {
        let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 1000, 1).unwrap();
        hist.auto(true);
        let hist2 = hist.clone();

        Self {
            total_time: hist,
            active_time: hist2,
        }
    }
}

//=================
// Info

pub struct Info {
    pub parents: HashMap<SpanGroup, Option<SpanGroup>>,
    pub timings: HashMap<SpanGroup, Timing>,
}

impl Info {
    fn new() -> Self {
        Self {
            parents: HashMap::new(),
            timings: HashMap::new(),
        }
    }
}

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    span_group: SpanGroup,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
    parent_group: Option<SpanGroup>,
}

//=================
// Latencies

type SpanGrouper =
    Arc<dyn Fn(&Option<SpanGroup>, &Attributes) -> SpanGroup + Send + Sync + 'static>;

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub struct Latencies {
    pub(crate) control: Control<Info, Info>,
    span_grouper: SpanGrouper,
}

impl Latencies {
    pub(crate) fn new(span_grouper: SpanGrouper) -> Latencies {
        Latencies {
            control: Control::new(Info::new(), op),
            span_grouper,
        }
    }

    pub fn with<V>(&self, f: impl FnOnce(&Info) -> V) -> V {
        self.control.with_acc(f).unwrap()
    }

    pub fn aggregate_results<G>(&self, _f: impl Fn(&SpanGroup) -> G) -> HashMap<G, Timing>
    where
        G: Eq + Hash,
    {
        todo!()
    }

    fn update_parents(&self, span_group: &SpanGroup, parent: &Option<SpanGroup>) {
        log::trace!(
            "entered `update_parents`for {:?} on {:?}",
            span_group,
            thread::current().id(),
        );
        self.control.with_tl_mut(&LOCAL_INFO, |info| {
            let parents = &mut info.parents;
            if parents.contains_key(span_group) {
                // Both local and global parents info are good for this callsite.
                return;
            }

            // Update local parents
            {
                parents.insert(span_group.clone(), parent.clone());
            }
        });
    }

    fn update_timings(&self, span_group: &SpanGroup, f: impl Fn(&mut Timing)) {
        self.control.with_tl_mut(&LOCAL_INFO,|info| {
            let  timings = &mut info.timings;
            let mut timing = timings
                .entry(span_group.clone())
                .or_insert_with(|| {
                    log::trace!(
                        "***** thread-loacal LocalCallsiteTiming created for callsite={:?} on thread={:?}",
                        span_group,
                        thread::current().id()
                    );
                    Timing::new()
                });

            f(&mut timing);
            log::trace!(
                "***** exiting with_local_callsite_info for callsite={:?} on thread={:?}",
                span_group,
                thread::current().id()
            );
        });
    }
}

impl<S> Layer<S> for Latencies
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_new_span`");
        let span = ctx.span(id).unwrap();

        let parent_span = span.parent();
        let parent_group = parent_span.map(|s| {
            s.extensions()
                .get::<SpanTiming>()
                .unwrap()
                .span_group
                .clone()
        });

        let span_group = (self.span_grouper)(&parent_group, attrs);

        span.extensions_mut().insert(SpanTiming {
            span_group,
            created_at: Instant::now(),
            entered_at: Instant::now(),
            acc_active_time: 0,
            parent_group,
        });
        log::trace!("`on_new_span` executed with id={:?}", id);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_enter` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.entered_at = Instant::now();
        log::trace!("`on_enter` executed with id={:?}", id);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_exit` wth span Id {:?}", id);
        let span = ctx.span(id).unwrap();
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.acc_active_time += (Instant::now() - span_timing.entered_at).as_micros() as u64;
        log::trace!("`on_exit` executed for span id {:?}", id);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_close` wth span Id {:?}", id);

        let span = ctx.span(&id).unwrap();
        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        self.update_timings(&span_timing.span_group, |r| {
            r.total_time
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
            r.active_time.record(span_timing.acc_active_time).unwrap();
        });

        log::trace!(
            "`on_close` completed call to with_local_callsite_info for span id {:?}",
            id
        );

        self.update_parents(&span_timing.span_group, &span_timing.parent_group);

        log::trace!("`on_close` executed for span id {:?}", id);
    }
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<Info, Info> = Holder::new(|| Info::new());
}

//=================
// functions

/// Used to accumulate results on [`Control`].
fn op(data: &Info, acc: &mut Info, tid: &ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    for (k, v) in data.parents.iter() {
        acc.parents.entry(k.clone()).or_insert_with(|| v.clone());
    }
    for (k, v) in data.timings.iter() {
        let timing = acc
            .timings
            .entry(k.clone())
            .or_insert_with(|| Timing::new());
        timing.total_time.add(v.total_time.clone()).unwrap();
        timing.active_time.add(v.active_time.clone()).unwrap();
    }
}
