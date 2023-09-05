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
// Callsite

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct Callsite {
    name: &'static str,
    code_line: String,
}

impl Callsite {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn code_line(&self) -> &str {
        &self.code_line
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct CallsitePriv {
    parent_callsite_id: Option<Identifier>,
    name: &'static str,
    code_line: String,
}

//=================
// SpanGroup

pub type Props = Vec<Vec<(&'static str, String)>>;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) callsite: Callsite,
    pub(crate) props: Props,
}

impl SpanGroup {
    fn new(sg_priv: SpanGroupPriv, callsite: Callsite) -> Self {
        SpanGroup {
            callsite: callsite,
            props: sg_priv.props,
        }
    }

    pub fn callsite(&self) -> &Callsite {
        &self.callsite
    }

    pub fn props(&self) -> &Props {
        &self.props
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupPriv {
    callsite_id: Identifier,
    props: Props,
}

impl SpanGroupPriv {
    fn new(attrs: &Attributes, props: Props) -> Self {
        SpanGroupPriv {
            callsite_id: attrs.metadata().callsite(),
            props,
        }
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

struct Info {
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

struct InfoPriv {
    pub callsites: HashMap<Identifier, CallsitePriv>,
    pub timings: HashMap<SpanGroupPriv, Timing>,
}

impl InfoPriv {
    fn new() -> Self {
        Self {
            callsites: HashMap::new(),
            timings: HashMap::new(),
        }
    }
}

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    // callsite_id: Identifier,
    // name: &'static str,
    // code_line: String,
    props: Vec<Vec<(&'static str, String)>>,
    // parent_callsite_id: Option<Identifier>,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
}

//=================
// Latencies

type SpanGrouper = Arc<dyn Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'static>;

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub struct Latencies {
    pub(crate) control: Control<InfoPriv, InfoPriv>,
    span_grouper: SpanGrouper,
    info: Arc<Info>,
}

impl Latencies {
    pub(crate) fn new(span_grouper: SpanGrouper) -> Latencies {
        Latencies {
            control: Control::new(InfoPriv::new(), op),
            span_grouper,
            info: Arc::new(Info::new()),
        }
    }

    pub fn with<V>(&self, f: impl FnOnce(&Info) -> V) -> V {
        // self.control.with_acc(f).unwrap()
        f(&self.info)
    }

    pub fn aggregate_timings<G>(&self, _f: impl Fn(&SpanGroup) -> G) -> HashMap<G, Timing>
    where
        G: Eq + Hash,
    {
        todo!()
    }

    fn update_callsites(&self, callsite_id: Identifier, callsite_priv: CallsitePriv) {
        log::trace!(
            "entered `update_callsites`for {:?} on {:?}",
            callsite_id,
            thread::current().id(),
        );
        self.control.with_tl_mut(&LOCAL_INFO, |info_priv| {
            let callsites = &mut info_priv.callsites;
            if callsites.contains_key(&callsite_id) {
                // Both local and global callsites map are good for this callsite.
                return;
            }

            // Update local callsites
            {
                callsites.insert(callsite_id, callsite_priv);
            }
        });
    }

    fn update_timings(&self, span_group_priv: SpanGroupPriv, f: impl Fn(&mut Timing)) {
        self.control.with_tl_mut(&LOCAL_INFO,|info| {
            let  timings = &mut info.timings;
            let mut timing = timings
                .entry(span_group_priv)
                .or_insert_with(|| {
                    log::trace!(
                        "***** thread-loacal LocalCallsiteTiming created for callsite={:?} on thread={:?}",
                        span_group_priv,
                        thread::current().id()
                    );
                    Timing::new()
                });

            f(&mut timing);
            log::trace!(
                "***** exiting with_local_callsite_info for callsite={:?} on thread={:?}",
                span_group_priv,
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
        // let meta = span.metadata();
        // let name = meta.name();
        // let callsite_id = meta.callsite();

        let parent_span = span.parent();
        let parent_span_timing = parent_span.map(|ps| ps.extensions().get::<SpanTiming>().unwrap());

        // let mut parent_callsite_id = None;
        let mut props = vec![(self.span_grouper)(attrs)];
        if let Some(parent_span_timing) = parent_span_timing {
            // parent_callsite_id = Some(parent_span_timing.callsite_id);
            props.append(&mut parent_span_timing.props);
        }

        span.extensions_mut().insert(SpanTiming {
            // callsite_id,
            // name,
            // code_line: format!("{}:{}", meta.module_path().unwrap(), meta.line().unwrap()),
            props,
            // parent_callsite_id,
            created_at: Instant::now(),
            entered_at: Instant::now(),
            acc_active_time: 0,
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
        let meta = span.metadata();
        let name = meta.name();
        let callsite_id = meta.callsite();
        let code_line = format!("{}:{}", meta.module_path().unwrap(), meta.line().unwrap());

        let parent_span = span.parent();
        let parent_callsite_id = parent_span.map(|ps| ps.metadata().callsite());

        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        let span_group_priv = SpanGroupPriv {
            callsite_id,
            props: span_timing.props,
        };
        self.update_timings(span_group_priv, |timing| {
            timing
                .total_time
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
            timing
                .active_time
                .record(span_timing.acc_active_time)
                .unwrap();
        });

        log::trace!(
            "`on_close` completed call to with_local_callsite_info for span id {:?}",
            id
        );

        let callsite_priv = CallsitePriv {
            parent_callsite_id,
            name,
            code_line,
        };

        self.update_callsites(callsite_id, callsite_priv);

        log::trace!("`on_close` executed for span id {:?}", id);
    }
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<InfoPriv, InfoPriv> = Holder::new(|| InfoPriv::new());
}

//=================
// functions

/// Used to accumulate results on [`Control`].
fn op(data: &InfoPriv, acc: &mut InfoPriv, tid: &ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    for (k, v) in data.callsites.iter() {
        acc.callsites.entry(k.clone()).or_insert_with(|| v.clone());
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
