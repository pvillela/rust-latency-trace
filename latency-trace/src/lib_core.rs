//! Core library implementation.

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
use tracing::{callsite::Identifier, span::Attributes, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

//=================
// Callsite

/// Provides [name](Self::name) and [code line](Self::code_line) information about where the tracing span was defined.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct CallsiteInfo {
    name: &'static str,
    code_line: String,
}

impl CallsiteInfo {
    /// Name of the tracing span.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Line of code where the tracing span was defined.
    pub fn code_line(&self) -> &str {
        &self.code_line
    }
}

fn callsite_id_to_usize(id: &Identifier) -> usize {
    let y = id.0 as *const dyn tracing::Callsite;
    y as *const () as usize
}

//=================
// SpanGroup

type CallsiteIdPath = Vec<Identifier>;
type PropsPath = Vec<Arc<Vec<(String, String)>>>;

/// Represents a set of [tracing::Span]s for which latency information should be collected as a group.
///
/// The coarsest-grained grouping of spans is characterized by a callsite (see [tracing::Callsite]), but
/// finer-grained groupings can be defined by adding a list of name-value pairs to the definition of a group.
/// Such a properties list can be computed from the span's [tracing::span::Attributes].
/// (Latency information can be aggregated across groupings of span groups by using
/// [Latencies::aggregate_timings].)
///
/// Span groups form a forest of trees where some pairs of span groups have a parent-child relationship.
/// This means that if SpanGroup A is the parent of SpanGroup B then, for each span that was assigned to group B,
/// its parent span
/// (see [Span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships))
/// was assigned to group A.
///
/// This struct holds the following information:
/// - [`callsite`](Self::callsite) information
/// - a [`props`](Self::props) field that contains the span group's list of name-value pairs (which may be empty)
/// - an [`idx`](Self::idx) field that uniquely characterizes the span group
/// - a [`parent_idx`](Self::parent_idx) field that is the `idx` field of the parent span group, if any.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) idx: usize,
    pub(crate) callsite: Arc<CallsiteInfo>,
    pub(crate) props: Arc<Vec<(String, String)>>,
    pub(crate) parent_idx: Option<usize>,
}

impl SpanGroup {
    /// Returns the span group's [CallsiteInfo].
    pub fn callsite(&self) -> &CallsiteInfo {
        &self.callsite
    }

    /// Returns the span group's properties list.
    ///
    /// This list can be empty as is the case with the default span grouper.
    pub fn props(&self) -> &Vec<(String, String)> {
        &self.props
    }

    /// Returns the callsite's name.
    pub fn name(&self) -> &'static str {
        self.callsite.name
    }

    /// Returns the callsite's file name and code line.
    pub fn code_line(&self) -> &str {
        &self.callsite.code_line()
    }

    /// Returns the span group's index in the list of all span groups.
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Returns the index of the span group's parent in the list of all span groups.
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
    callsite_uid_path: Vec<usize>,
    props_path: PropsPath,
}

//=================
// Timing

/// Holds latency timing information for a given item, e.g., a span group or a group of span groups.
///
/// Information is held in two fields of type `T`:
/// - [`total_time`](Self::total_time), with latency information including async waits
/// - [`active_time`](Self::active_time), with latency information excluding async waits
///
/// The default type parameterization is the [`Timing`] type.
#[derive(Clone, Debug)]
pub struct TimingView<T> {
    pub(crate) total_time: T,
    pub(crate) active_time: T,
}

/// This is the default [`TimingView`] used to capture the collected latency information in **microseconds**.
///
/// It can be transformed into other timing views by using the [TimingView::map] method.
pub type Timing = TimingView<Histogram<u64>>;

impl<T> TimingView<T> {
    /// Returns latency info including async waits.
    pub fn total_time(&self) -> &T {
        &self.total_time
    }

    /// Returns latency info excluding async waits.
    pub fn active_time(&self) -> &T {
        &self.active_time
    }

    /// Transforms the [`total_time`](Self::total_time) and [`active_time`](Self::active_time)
    /// latency info into a target [`TimingView<U>`].
    pub fn map<U>(&self, f: impl Fn(&T) -> U) -> TimingView<U> {
        TimingView {
            total_time: f(&self.total_time),
            active_time: f(&self.active_time),
        }
    }
}

impl Timing {
    /// Constructs a [`Timing`].
    fn new(hist_high: u64, hist_sigfig: u8) -> Self {
        let mut hist = Histogram::<u64>::new_with_bounds(1, hist_high, hist_sigfig).unwrap();
        hist.auto(true);
        let hist2 = hist.clone();

        Self {
            total_time: hist,
            active_time: hist2,
        }
    }
}

//=================
// Latencies

/// Provides the list of [`SpanGroup`]s and  a mapping from the span groups to the [`Timing`] information
/// collected for them.
///
/// The span groups are ordered such that parent span groups appear before their children.

#[derive(Debug)]
pub struct Latencies {
    pub(crate) span_groups: Vec<SpanGroup>,
    pub(crate) timings: BTreeMap<SpanGroup, Timing>,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

impl Latencies {
    /// Returns the list of [`SpanGroup`]s, ordered such that parent span groups appear before their children.
    pub fn span_groups(&self) -> &Vec<SpanGroup> {
        &self.span_groups
    }

    /// Returns a mapping from the span groups to the [`Timing`] information
    /// collected for them. The span groups are ordered such that parent span groups appear before their children.
    pub fn timings(&self) -> &BTreeMap<SpanGroup, Timing> {
        &self.timings
    }

    /// Aggregates span group [`Timing`]s by sets of span groups that have the same value when `f` is applied.
    pub fn aggregate_timings<G>(&self, f: impl Fn(&SpanGroup) -> G) -> BTreeMap<G, Timing>
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Timing> = BTreeMap::new();
        for (k, v) in &self.timings {
            let g = f(k);
            let timing = match res.get_mut(&g) {
                Some(timing) => timing,
                None => {
                    res.insert(g.clone(), Timing::new(self.hist_high, self.hist_sigfig));
                    res.get_mut(&g).unwrap()
                }
            };
            timing.total_time.add(v.total_time()).unwrap();
            timing.active_time.add(v.active_time()).unwrap();
        }
        res
    }
}

pub(crate) struct LatenciesPriv {
    callsites: HashMap<Identifier, Arc<CallsiteInfo>>,
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
// LatencyTraceCfg

pub(crate) struct LatencyTraceCfg {
    pub(crate) span_grouper:
        Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

impl LatencyTraceCfg {
    /// Used to accumulate results on [`Control`].
    fn op(&self) -> impl Fn(LatenciesPriv, &mut LatenciesPriv, &ThreadId) + Send + Sync + 'static {
        let hist_high = self.hist_high;
        let hist_sigfig = self.hist_sigfig;
        move |data: LatenciesPriv, acc: &mut LatenciesPriv, tid: &ThreadId| {
            log::debug!("executing `op` for {:?}", tid);
            let callsites = data.callsites;
            let timings = data.timings;
            for (k, v) in callsites {
                acc.callsites.entry(k).or_insert_with(|| v);
            }
            for (k, v) in timings {
                let timing = acc
                    .timings
                    .entry(k)
                    .or_insert_with(|| Timing::new(hist_high, hist_sigfig));
                timing.total_time.add(v.total_time).unwrap();
                timing.active_time.add(v.active_time).unwrap();
            }
        }
    }
}

//=================
// LatencyTracePriv

/// Provides access to [Timings] containing the latencies collected for different span groups.
#[derive(Clone)]
pub(crate) struct LatencyTracePriv {
    pub(crate) control: Control<LatenciesPriv, LatenciesPriv>,
    span_grouper: Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>,
    hist_high: u64,
    hist_sigfig: u8,
}

impl LatencyTracePriv {
    pub(crate) fn new(config: LatencyTraceCfg) -> LatencyTracePriv {
        LatencyTracePriv {
            control: Control::new(LatenciesPriv::new(), config.op()),
            span_grouper: config.span_grouper,
            hist_high: config.hist_high,
            hist_sigfig: config.hist_sigfig,
        }
    }

    fn ensure_callsites_updated(
        &self,
        callsite_id: Identifier,
        callsite_fn: impl FnOnce() -> Arc<CallsiteInfo>,
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
        self.control.with_tl_mut(&LOCAL_INFO, |lp| {
            let timings = &mut lp.timings;
            let mut timing = {
                if let Some(timing) = timings.get_mut(span_group_priv) {
                    timing
                } else {
                    log::trace!(
                        "thread-loacal Timing created for {:?} on {:?}",
                        span_group_priv,
                        thread::current().id()
                    );
                    timings.insert(
                        span_group_priv.clone(),
                        Timing::new(self.hist_high, self.hist_sigfig),
                    );
                    timings.get_mut(span_group_priv).unwrap()
                }
            };

            f(&mut timing);

            log::trace!(
                "exiting `update_timings` for {:?} on {:?}",
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
                let callsite_uid_path = sgp
                    .callsite_id_path
                    .iter()
                    .map(|cid| callsite_id_to_usize(cid))
                    .collect::<Vec<usize>>();
                let sgt = SpanGroupTemp {
                    callsite_uid_path,
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
        lp: &LatenciesPriv,
        sgt_to_sgp: BTreeMap<SpanGroupTemp, SpanGroupPriv>,
    ) -> (Vec<SpanGroup>, HashMap<SpanGroupPriv, usize>) {
        let mut idx = 0;
        let mut sgp_to_idx: HashMap<SpanGroupPriv, usize> = HashMap::new();
        let mut span_groups: Vec<SpanGroup> = Vec::with_capacity(sgt_to_sgp.len());
        sgt_to_sgp.into_iter().for_each(|(_, sgp)| {
            let cid = sgp.callsite_id_path.last().unwrap();
            let sg = SpanGroup {
                callsite: lp.callsites.get(cid).unwrap().clone(),
                props: sgp.props_path.last().unwrap().clone(),
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
        &self,
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
            hist_high: self.hist_high,
            hist_sigfig: self.hist_sigfig,
        }
    }

    /// Generates the publicly accessible [`Latencies`] as in post-processing after all thread-local
    /// data has been accumulated.
    pub(crate) fn generate_latencies(&self) -> Latencies {
        self.control
            .with_acc(|lp| {
                let sgt_to_sgp = Self::to_latencies_1(lp);
                let (span_groups, sgp_to_idx) = Self::to_latencies_2(lp, sgt_to_sgp);
                self.to_latencies_3(lp, span_groups, sgp_to_idx)
            })
            .unwrap()
    }
}

impl<S> Layer<S> for LatencyTracePriv
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        log::trace!("`on_new_span` start: name={}, id={:?}", span.name(), id);
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

        log::trace!("`on_new_span` end: name={}, id={:?}", span.name(), id);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        log::trace!("`on_enter` start: name={}, id={:?}", span.name(), id);
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.entered_at = Instant::now();
        log::trace!("`on_enter` end: name={}, id={:?}", span.name(), id);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        log::trace!("`on_exit` start: name={}, id={:?}", span.name(), id);
        let mut ext = span.extensions_mut();
        let span_timing = ext.get_mut::<SpanTiming>().unwrap();
        span_timing.acc_active_time += (Instant::now() - span_timing.entered_at).as_micros() as u64;
        log::trace!("`on_exit` end: name={}, id={:?}", span.name(), id);
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        log::trace!("`on_close` start: name={}, id={:?}", span.name(), id);
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
            "`on_close` completed call to update_timings: name={}, id={:?}",
            span.name(),
            id
        );

        self.ensure_callsites_updated(callsite_id, || {
            let name = meta.name();
            let code_line = format!("{}:{}", meta.file().unwrap(), meta.line().unwrap());
            Arc::new(CallsiteInfo { name, code_line })
        });

        log::trace!("`on_close` end: name={}, id={:?}", span.name(), id);
    }
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<LatenciesPriv, LatenciesPriv> = Holder::new(|| LatenciesPriv::new());
}
