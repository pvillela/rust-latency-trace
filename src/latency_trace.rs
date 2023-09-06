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
    collections::{BTreeMap, HashMap},
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
        self.name
    }

    pub fn code_line(&self) -> &str {
        &self.code_line
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct CallsitePriv {
    pub(crate) parent_callsite_id: Option<Identifier>,
    pub(crate) name: &'static str,
    pub(crate) code_line: String,
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
    pub fn callsite(&self) -> &Callsite {
        &self.callsite
    }

    pub fn props(&self) -> &Props {
        &self.props
    }

    pub fn name(&self) -> &str {
        self.callsite.name
    }

    pub fn code_line(&self) -> &str {
        &self.callsite.code_line()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupPriv {
    callsite_id: Identifier,
    props: Props,
}

//=================
// Timing

#[derive(Clone, Debug)]
pub struct Timing {
    pub total_time: Histogram<u64>,
    pub active_time: Histogram<u64>,
}

impl Timing {
    pub fn new() -> Self {
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

pub type Latencies = BTreeMap<SpanGroup, SpanGroupInfo>;

#[derive(Clone)]
pub struct SpanGroupInfo {
    parent: Option<SpanGroup>,
    timing: Timing,
}

impl SpanGroupInfo {
    pub fn parent(&self) -> Option<&SpanGroup> {
        self.parent.as_ref()
    }

    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    pub fn total_time(&self) -> &Histogram<u64> {
        &self.timing.total_time
    }

    pub fn active_time(&self) -> &Histogram<u64> {
        &self.timing.active_time
    }
}

pub(crate) struct InfoPriv {
    callsites: HashMap<Identifier, CallsitePriv>,
    timings: HashMap<SpanGroupPriv, Timing>,
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
// LatencyTrace

type SpanGrouper = Arc<dyn Fn(&Attributes) -> Vec<(&'static str, String)> + Send + Sync + 'static>;

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub(crate) struct LatencyTrace {
    pub(crate) control: Control<InfoPriv, InfoPriv>,
    span_grouper: SpanGrouper,
}

impl LatencyTrace {
    pub(crate) fn new(span_grouper: SpanGrouper) -> LatencyTrace {
        LatencyTrace {
            control: Control::new(InfoPriv::new(), op),
            span_grouper,
        }
    }

    fn ensure_callsites_updated(
        &self,
        callsite_id: Identifier,
        callsite_priv_fn: impl FnOnce() -> CallsitePriv,
    ) {
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
                callsites.insert(callsite_id, callsite_priv_fn());
            }
        });
    }

    fn update_timings(&self, span_group_priv: &SpanGroupPriv, f: impl Fn(&mut Timing)) {
        self.control.with_tl_mut(&LOCAL_INFO, |info| {
            let timings = &mut info.timings;
            let mut timing = {
                if let Some(timing) = timings.get_mut(span_group_priv) {
                    timing
                } else {
                    log::trace!(
                        "***** thread-loacal Timing created for {:?} on {:?}",
                        span_group_priv,
                        thread::current().id()
                    );
                    timings.insert(span_group_priv.clone(), Timing::new());
                    timings.get_mut(span_group_priv).unwrap()
                }
            };

            f(&mut timing);

            log::trace!(
                "***** exiting `update_timings` for {:?} on {:?}",
                span_group_priv,
                thread::current().id()
            );
        });
    }

    /// Helper to `generate_latencies`.
    fn to_span_group_info_pair(
        info_priv: &InfoPriv,
        sg_priv: &SpanGroupPriv,
    ) -> (SpanGroup, SpanGroupInfo) {
        let callsite_id = &sg_priv.callsite_id;
        let callsite_priv = info_priv.callsites.get(callsite_id).unwrap();
        let props = sg_priv.props.clone();
        let span_group = SpanGroup {
            callsite: Callsite {
                name: callsite_priv.name,
                code_line: callsite_priv.code_line.clone(),
            },
            props: props.clone(),
        };
        let timing = info_priv.timings.get(sg_priv).unwrap().clone();

        let parent_id = callsite_priv.parent_callsite_id.as_ref();
        let parent = match parent_id {
            None => None,
            Some(parent_id) => {
                let parent_callsite_priv = info_priv.callsites.get(parent_id).unwrap();
                let parent_callsite = Callsite {
                    name: parent_callsite_priv.name,
                    code_line: parent_callsite_priv.code_line.clone(),
                };
                let parent_props = Vec::from(&props[1..]);

                Some(SpanGroup {
                    callsite: parent_callsite,
                    props: parent_props,
                })
            }
        };

        let info = SpanGroupInfo { parent, timing };

        (span_group, info)
    }

    /// Generates the publicly accessible [`Latencies`] as a post-processing step after all thread-local
    /// data has been accumulated.
    pub(crate) fn generate_latencies(&self) -> Latencies {
        self.control
            .with_acc(|info_priv| {
                let sg_privs = info_priv.timings.keys();
                let pairs =
                    sg_privs.map(|sg_priv| Self::to_span_group_info_pair(info_priv, sg_priv));
                pairs.collect::<Latencies>()
            })
            .unwrap()
    }
}

impl<S> Layer<S> for LatencyTrace
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        log::trace!("entered `on_new_span`");
        let span = ctx.span(id).unwrap();

        let parent_span = span.parent();
        let parent_span_props =
            parent_span.map(|ps| ps.extensions().get::<SpanTiming>().unwrap().props.clone());

        let mut props = vec![(self.span_grouper)(attrs)];
        if let Some(mut x) = parent_span_props {
            props.append(&mut x);
        }

        span.extensions_mut().insert(SpanTiming {
            props,
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
        let callsite_id = meta.callsite();
        let code_line = format!("{}:{}", meta.file().unwrap(), meta.line().unwrap());

        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        let span_group_priv = SpanGroupPriv {
            callsite_id: callsite_id.clone(),
            props: span_timing.props.clone(),
        };

        self.update_timings(&span_group_priv, |timing| {
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

        let callsite_priv_fn = || {
            let name = meta.name();
            let parent_span = span.parent();
            let parent_callsite_id = parent_span.map(|ps| ps.metadata().callsite());

            CallsitePriv {
                parent_callsite_id,
                name,
                code_line,
            }
        };

        self.ensure_callsites_updated(callsite_id, callsite_priv_fn);

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
