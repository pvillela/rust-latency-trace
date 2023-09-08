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

type CallsiteIdPath = Vec<Identifier>;
type CallsitePath = Vec<Arc<Callsite>>;
type PropsPath = Vec<Arc<Vec<(String, String)>>>;

#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) idx: usize,
    pub(crate) callsite: Callsite,
    pub(crate) props: Vec<(String, String)>,
    pub(crate) parent_idx: Option<usize>,
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

    pub fn parent_idx(&self) -> Option<usize> {
        self.parent_idx
    }
}

/// Private form of spangroup used during trace collection, more efficient than [`SpanGroup`] for trace
/// data collection.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupPriv {
    /// callsite ID of the span group followed by the callsite IDs of its ancestors.
    callsite_id_path: CallsiteIdPath,

    /// Properties of the span group followed by the properties of its ancestors.
    props_path: PropsPath,
}

/// Intermediate form of SpanGroup that is sortable and ensures that parents always appear before
/// children in sort order.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
struct SpanGroupTemp {
    callsite_path: CallsitePath,
    props_path: PropsPath,
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
    callsites: HashMap<Identifier, Arc<Callsite>>,
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
    callsite_id_path: CallsiteIdPath,
    props_path: PropsPath,
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
        callsite_fn: impl FnOnce() -> Arc<Callsite>,
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

    /// Step in transforming the accumulated data in Control into the [`Latencies`] output.
    /// Due to their structure, SpanGroupTemp is sortable and ensures that parents always appear before
    /// children in sort order.
    fn to_latencies_1(lp: &LatenciesPriv) -> BTreeMap<SpanGroupTemp, SpanGroupPriv> {
        lp.timings
            .keys()
            .map(|sgp| {
                let callsite_path = sgp
                    .callsite_id_path
                    .iter()
                    .map(|cid| lp.callsites.get(cid).unwrap().clone())
                    .collect::<CallsitePath>();
                let sgt = SpanGroupTemp {
                    callsite_path,
                    props_path: sgp.props_path.clone(),
                };
                (sgt, sgp.clone())
            })
            .collect()
    }

    /// Step in transforming the accumulated data in Control into the [`Latencies`] output.
    /// Generages a vector os [SpanGroup]s, each with its `idx` but without `parent_idx`, as well as
    /// a map from [SpanGroupPriv] to the corresponding idx in the vector.
    /// Since the input is ordered so that parents always appear before their children, the resulting
    /// [`SpanGroup`] instances have the same ordering, which is reflected by their `idx` field and
    /// corresopnds to their position in the output vector.
    fn to_latencies_2(
        sgt_to_sgp: BTreeMap<SpanGroupTemp, SpanGroupPriv>,
    ) -> (Vec<SpanGroup>, HashMap<SpanGroupPriv, usize>) {
        let mut idx = 0;
        let mut sgp_to_idx: HashMap<SpanGroupPriv, usize> = HashMap::new();
        let mut span_groups: Vec<SpanGroup> = Vec::with_capacity(sgt_to_sgp.len());
        sgt_to_sgp.into_iter().for_each(|(sgt, sgp)| {
            let sg = SpanGroup {
                callsite: sgt.callsite_path.last().unwrap().as_ref().clone(),
                props: sgp.props_path.last().unwrap().as_ref().clone(),
                idx,
                parent_idx: None,
            };
            span_groups.push(sg);
            sgp_to_idx.insert(sgp, idx);
            idx += 1;
        });
        (span_groups, sgp_to_idx)
    }

    /// Step in transforming the accumulated data in Control into the [`Latencies`] output.
    /// Adds the `parent_idx`to each [SpanGroup] in `span_groups` and produces the [Latencies] output.
    fn to_latencies_3(
        lp: &LatenciesPriv,
        mut span_groups: Vec<SpanGroup>,
        sgp_to_idx: HashMap<SpanGroupPriv, usize>,
    ) -> Latencies {
        // Add parent_idx to items in `span_groups`
        for (sgp, idx) in sgp_to_idx.iter() {
            let path_len = sgp.callsite_id_path.len();
            let parent_sgp = if path_len == 1 {
                None
            } else {
                Some(SpanGroupPriv {
                    callsite_id_path: Vec::from(&sgp.callsite_id_path[..path_len - 1]),
                    props_path: Vec::from(&sgp.props_path[..path_len - 1]),
                })
            };
            let parent_idx = parent_sgp.map(|psgp| *sgp_to_idx.get(&psgp).unwrap());

            let sg = &mut span_groups[*idx];
            sg.parent_idx = parent_idx;
        }

        let timings: BTreeMap<SpanGroup, Timing> = lp
            .timings
            .iter()
            .map(|(sgp, timing)| {
                let idx = *sgp_to_idx.get(sgp).unwrap();
                let sg = &span_groups[idx];
                (sg.clone(), timing.clone())
            })
            .collect();

        Latencies {
            span_groups,
            timings,
        }
    }

    /// Generates the publicly accessible [`Latencies`] as in post-processing after all thread-local
    /// data has been accumulated.
    pub(crate) fn generate_latencies(&self) -> Latencies {
        self.control
            .with_acc(|lp| {
                let sgt_to_sgp = Self::to_latencies_1(lp);
                let (span_groups, sgp_to_idx) = Self::to_latencies_2(sgt_to_sgp);
                Self::to_latencies_3(lp, span_groups, sgp_to_idx)
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

        let callsite_id = span.metadata().callsite();
        let props = (self.span_grouper)(attrs);
        let (callsite_id_path, props_path) = match parent_span {
            None => (vec![callsite_id], vec![Arc::new(props)]),
            Some(parent_span) => {
                let ext = parent_span.extensions();
                let pst = ext.get::<SpanTiming>().unwrap();
                let mut callsite_id_path = pst.callsite_id_path.clone();
                callsite_id_path.push(callsite_id);
                let mut props_path = pst.props_path.clone();
                props_path.push(Arc::new(props));
                (callsite_id_path, props_path)
            }
        };

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
            Arc::new(Callsite { name, code_line })
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
    for (k, v) in callsites {
        acc.callsites.entry(k).or_insert_with(|| v);
    }
    for (k, v) in timings {
        let timing = acc.timings.entry(k).or_insert_with(|| Timing::new());
        timing.total_time.add(v.total_time).unwrap();
        timing.active_time.add(v.active_time).unwrap();
    }
}
