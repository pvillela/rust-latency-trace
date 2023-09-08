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
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn code_line(&self) -> &str {
        &self.code_line
    }
}

//=================
// SpanGroup

pub type Props = Vec<Vec<(String, String)>>;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) callsite: Callsite,
    pub(crate) props: Vec<(String, String)>,
    pub(crate) idx: usize,
    pub(crate) parent_idx: usize,
}

impl SpanGroup {
    pub fn callsite(&self) -> &Callsite {
        &self.callsite
    }

    pub fn props(&self) -> &Vec<(String, String)> {
        &self.props
    }

    pub fn name(&self) -> &'static str {
        self.callsite.name
    }

    pub fn code_line(&self) -> &str {
        &self.callsite.code_line()
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn parent_idx(&self) -> usize {
        self.parent_idx
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupPriv {
    /// callsite ID of the span group followed by the callsite IDs of its ancestors.
    callsite_id_path: Vec<Identifier>,

    /// Properties of the span group followed by the properties of its ancestors.
    props_path: Props,
}

//=================
// Timing

#[derive(Clone, Debug)]
pub struct Timing {
    pub(crate) total_time: Histogram<u64>,
    pub(crate) active_time: Histogram<u64>,
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

    pub fn total_time(&self) -> &Histogram<u64> {
        &self.total_time
    }

    pub fn active_time(&self) -> &Histogram<u64> {
        &self.active_time
    }
}

//=================
// Latencies

pub struct Latencies {
    pub span_groups: Vec<SpanGroup>,
    pub timings: BTreeMap<SpanGroup, Timing>,
}

pub(crate) struct LatenciesPriv {
    callsites: HashMap<Identifier, Callsite>,
    timings: HashMap<SpanGroupPriv, Timing>,
}

impl LatenciesPriv {
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
    callsite_id_path: Vec<Identifier>,
    props_path: Vec<Vec<(String, String)>>,
    created_at: Instant,
    entered_at: Instant,
    acc_active_time: u64,
}

//=================
// LatencyTrace

type SpanGrouper = Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>;

/// Provides access a [Timings] containing the latencies collected for different span callsites.
#[derive(Clone)]
pub(crate) struct LatencyTrace {
    pub(crate) control: Control<LatenciesPriv, LatenciesPriv>,
    span_grouper: SpanGrouper,
}

impl LatencyTrace {
    pub(crate) fn new(span_grouper: SpanGrouper) -> LatencyTrace {
        LatencyTrace {
            control: Control::new(LatenciesPriv::new(), op),
            span_grouper,
        }
    }

    fn ensure_callsites_updated(
        &self,
        callsite_id: Identifier,
        callsite_fn: impl FnOnce() -> Callsite,
    ) {
        log::trace!(
            "entered `ensure_callsites_updated`for {:?} on {:?}",
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
                callsites.insert(callsite_id, callsite_fn());
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

    fn to_latencies_1(lp: &LatenciesPriv) -> HashMap<SpanGroupPriv, SpanGroup> {
        let mut idx = 0;
        lp.timings
            .keys()
            .map(|sgp| {
                let sg = SpanGroup {
                    callsite: lp.callsites.get(&sgp.callsite_id_path[0]).unwrap().clone(),
                    props: sgp.props_path[0].clone(),
                    idx,
                    parent_idx: usize::MAX,
                };
                idx += 1;
                (sgp.clone(), sg)
            })
            .collect()
    }

    fn to_latencies_2(
        sgp_to_sg: HashMap<SpanGroupPriv, SpanGroup>,
    ) -> Vec<Option<(SpanGroupPriv, SpanGroup)>> {
        let mut spg_sg_pairs = vec![None; sgp_to_sg.len()];

        sgp_to_sg.iter().for_each(|(sgp, sg)| {
            let sgp_parent = SpanGroupPriv {
                callsite_id_path: Vec::from(&sgp.callsite_id_path[1..]),
                props_path: Vec::from(&sgp.props_path[1..]),
            };
            let parent_idx = sgp_to_sg.get(&sgp_parent).unwrap().idx;
            let mut sg = sg.clone();
            let idx = sg.idx;
            sg.parent_idx = parent_idx;
            spg_sg_pairs[idx] = Some((sgp.clone(), sg));
        });

        spg_sg_pairs
    }

    fn to_latencies_3(
        lp: &LatenciesPriv,
        spg_sg_pairs: Vec<Option<(SpanGroupPriv, SpanGroup)>>,
    ) -> Latencies {
        let mut span_groups = vec![];
        span_groups.reserve_exact(spg_sg_pairs.len());
        let timings: BTreeMap<SpanGroup, Timing> = spg_sg_pairs
            .into_iter()
            .map(|opt_pair| {
                let (sgp, sg) = opt_pair.unwrap();
                (sg, lp.timings.get(&sgp).unwrap().clone())
            })
            .collect();

        Latencies {
            span_groups,
            timings,
        }
    }

    // /// Helper to `generate_latencies`.
    // fn to_span_group_timing_pair(
    //     info_priv: &LatenciesPriv,
    //     sg_priv: &SpanGroupPriv,
    // ) -> (SpanGroup, Timing) {
    //     let props_path = &sg_priv.props_path;
    //     let callsite_path: Vec<Callsite> = sg_priv
    //         .callsite_id_path
    //         .iter()
    //         .map(|id| info_priv.callsites.get(id).unwrap().clone())
    //         .collect();
    //     let timing = info_priv.timings.get(sg_priv).unwrap();

    //     let span_group = SpanGroup {
    //         callsite_path,
    //         props_path: props_path.clone(),
    //     };

    //     (span_group, timing.clone())
    // }

    /// Generates the publicly accessible [`Latencies`] as a post-processing step after all thread-local
    /// data has been accumulated.
    pub(crate) fn generate_latencies(&self) -> Latencies {
        self.control
            .with_acc(|lp| {
                // let sg_privs = lp.timings.keys();
                // let pairs = sg_privs.map(|sg_priv| Self::to_span_group_timing_pair(lp, sg_priv));
                // pairs.collect::<Latencies>()
                let sgp_to_sg = Self::to_latencies_1(lp);
                let spg_sg_pairs = Self::to_latencies_2(sgp_to_sg);
                Self::to_latencies_3(lp, spg_sg_pairs)
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

        let mut callsite_id_path = vec![span.metadata().callsite()];
        let mut props_path = vec![(self.span_grouper)(attrs)];
        if let Some(parent_span) = parent_span {
            let ext = parent_span.extensions();
            let pst = ext.get::<SpanTiming>().unwrap();
            callsite_id_path.append(&mut pst.callsite_id_path.clone());
            props_path.append(&mut pst.props_path.clone());
        }

        span.extensions_mut().insert(SpanTiming {
            callsite_id_path,
            props_path,
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

        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        let span_group_priv = SpanGroupPriv {
            callsite_id_path: span_timing.callsite_id_path.clone(),
            props_path: span_timing.props_path.clone(),
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
            "`on_close` completed call to update_timings for span id {:?}",
            id
        );

        self.ensure_callsites_updated(callsite_id, || {
            let name = meta.name();
            let code_line = format!("{}:{}", meta.file().unwrap(), meta.line().unwrap());
            Callsite { name, code_line }
        });

        log::trace!("`on_close` executed for span id {:?}", id);
    }
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<LatenciesPriv, LatenciesPriv> = Holder::new(|| LatenciesPriv::new());
}

//=================
// Functions

/// Used to accumulate results on [`Control`].
fn op(data: LatenciesPriv, acc: &mut LatenciesPriv, tid: &ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    let callsites = data.callsites;
    let timings = data.timings;
    for (k, v) in callsites.into_iter() {
        acc.callsites.entry(k).or_insert_with(|| v);
    }
    for (k, v) in timings.into_iter() {
        let timing = acc.timings.entry(k).or_insert_with(|| Timing::new());
        timing.total_time.add(v.total_time).unwrap();
        timing.active_time.add(v.active_time).unwrap();
    }
}
