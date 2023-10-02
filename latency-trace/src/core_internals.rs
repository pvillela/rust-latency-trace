//! Core library implementation.

use base64ct::{Base64, Encoding};
use hdrhistogram::Histogram;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use thread_local_drop::{self, Control, ControlLock, Holder};
use tracing::{span::Attributes, Id, Subscriber};
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

//=================
// SpanGroup

type CallsiteInfoPath = Arc<Vec<Arc<CallsiteInfo>>>;
type Props = Vec<(String, String)>;
type PropsPath = Arc<Vec<Arc<Props>>>;

/// Represents a set of [tracing::Span]s for which latency information should be collected as a group. It is
/// the unit of latency information collection.
///
/// Spans are defined in the code using macros and functions from the Rust [tracing](https://crates.io/crates/tracing) library which define span ***callsite***s, i.e., the places in the code where spans are defined. As the code is executed, a span definition in the code may be executed multiple times -- each such execution is a span instance. Span instances arising from the same span definition are grouped into [`SpanGroup`]s for latency information collection. Latencies are collected using [Histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html)s from the [hdrhistogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/) library.
///
/// The grouping of spans for latency collection is not exactly based on the span definitions in the code. Spans at runtime are structured as a set of [span trees](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships) that correspond to the nesting of spans from code execution paths. The grouping of runtime spans for latency collection should respect the runtime parent-child relationships among spans.
///
/// Thus, [`SpanGroup`]s form a forest of trees where some pairs of span groups have a parent-child relationship, corresponding to the parent-child relationships of the spans associated with the span groups. This means that if `SpanGroup A` is the parent of `SpanGroup B` then, for each span that was assigned to group `B`, its parent span was assigned to group `A`.
///
/// The coarsest-grained grouping of spans is characterized by a ***callsite path*** -- a callsite and the (possibly empty) list of its ancestor callsites based on the different runtime execution paths (see [Span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships)). This is the default `SpanGroup` definition. Finer-grained groupings of spans can differentiate groups of spans with the same callsite path by taking into account values computed at runtime from the spans' runtime [Attributes](https://docs.rs/tracing/0.1.37/tracing/span/struct.Attributes.html).
///
/// This struct holds the following information:
/// - [`callsite`](Self::callsite) information
/// - a [`props`](Self::props) field that contains the span group's list of name-value pairs (which may be empty)
/// - an [`id`](Self::id) field that uniquely characterizes the span group
/// - a [`parent_id`](Self::parent_id) field that is the `id` field of the parent span group, if any.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    pub(crate) callsite: Arc<CallsiteInfo>,
    pub(crate) props: Arc<Props>,
    pub(crate) depth: usize,
    pub(crate) id: Arc<str>,
    pub(crate) parent_id: Option<Arc<str>>,
}

impl SpanGroup {
    /// Returns the span group's [CallsiteInfo].
    pub fn callsite(&self) -> &CallsiteInfo {
        &self.callsite
    }

    /// Returns the span group's properties list.
    ///
    /// This list can be empty as is the case with the default span grouper.
    pub fn props(&self) -> &[(String, String)] {
        &self.props
    }

    /// Returns the callsite's name.
    pub fn name(&self) -> &'static str {
        self.callsite.name
    }

    /// Returns the callsite's file name and code line.
    pub fn code_line(&self) -> &str {
        self.callsite.code_line()
    }

    /// Returns the span group's ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the ID of the span group's parent.
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.iter().map(|x| x.as_ref()).next()
    }
}

/// Private form of spangroup used during trace collection, more efficient than [`SpanGroup`] for trace
/// data collection.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct SpanGroupPriv {
    /// Callsite info of the span group preceded by the callsite IDs of its ancestors.
    callsite_info_path: CallsiteInfoPath,

    /// Properties of the span group preceded by the properties of its ancestors.
    props_path: PropsPath,
}

impl SpanGroupPriv {
    fn parent(&self) -> Option<Self> {
        let len = self.callsite_info_path.len();
        if len == 1 {
            return None;
        }
        Some(SpanGroupPriv {
            callsite_info_path: Arc::new(self.callsite_info_path[0..len - 1].into()),
            props_path: Arc::new(self.props_path[0..len - 1].into()),
        })
    }
}

//=================
// Timing

/// Alias of [`Histogram<u64>`].
pub type Timing = Histogram<u64>;

/// Constructs a [`Timing`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
fn new_timing(hist_high: u64, hist_sigfig: u8) -> Timing {
    let mut hist = Histogram::<u64>::new_with_bounds(1, hist_high, hist_sigfig).unwrap();
    hist.auto(true);
    hist
}

//=================
// Timings

/// Mapping of [SpanGroup]s to the [Timing] information recorded for them.
pub type Timings = BTreeMap<SpanGroup, Timing>;

/// Adds methods to [`Timings`];
pub trait TimingsExt {
    /// Combines histograms of span group according to sets of span groups that yield the same value when `f`
    /// is applied. The values resulting from applying `f` to span groups are called ***aggregate key***s and
    /// the sets of span groups corresponding to each *aggregate key* are called ***aggregates***.
    ///
    /// An aggregation is consistent if and only if, for each *aggregate*, all the span groups in the *aggregate*
    /// have the same callsite.
    ///
    /// This function returns a pair with the following components:
    /// - a [BTreeMap] that associates each *aggregate key* to its aggregated histogram;
    /// - a boolean that is `true` if the aggregation is consistent, `false` otherwise.
    fn aggregate<G>(&self, f: impl Fn(&SpanGroup) -> G) -> (BTreeMap<G, Histogram<u64>>, bool)
    where
        G: Ord + Clone;

    /// Returns a map from span group ID to [`SpanGroup`].
    fn id_to_span_group(&self) -> BTreeMap<String, SpanGroup>;

    /// Returns a map from [`SpanGroup`] to its parent.
    fn span_group_to_parent(&self) -> BTreeMap<SpanGroup, Option<SpanGroup>>;
}

impl TimingsExt for Timings {
    fn aggregate<G>(&self, f: impl Fn(&SpanGroup) -> G) -> (BTreeMap<G, Histogram<u64>>, bool)
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Histogram<u64>> = BTreeMap::new();
        let mut aggregates: BTreeMap<G, Arc<CallsiteInfo>> = BTreeMap::new();
        let mut aggregates_are_consistent = true;
        for (k, v) in self {
            // Construct aggregation map.
            let g = f(k);
            let hist = match res.get_mut(&g) {
                Some(hist) => hist,
                None => {
                    res.insert(g.clone(), Histogram::new_from(v));
                    res.get_mut(&g).unwrap()
                }
            };
            hist.add(v).unwrap();

            // Check aggregation consistency.
            if aggregates_are_consistent {
                aggregates_are_consistent = match aggregates.get(&g) {
                    Some(callsite) => callsite.as_ref() == k.callsite(),
                    None => {
                        aggregates.insert(g.clone(), k.callsite.clone());
                        true
                    }
                };
            }
        }

        (res, aggregates_are_consistent)
    }

    fn id_to_span_group(&self) -> BTreeMap<String, SpanGroup> {
        self.keys()
            .map(|k| (k.id().to_owned(), k.clone()))
            .collect()
    }

    fn span_group_to_parent(&self) -> BTreeMap<SpanGroup, Option<SpanGroup>> {
        let id_to_sg = self.id_to_span_group();
        self.keys()
            .map(|sg| {
                let parent = sg.parent_id().map(|pid| id_to_sg.get(pid).unwrap().clone());
                (sg.clone(), parent)
            })
            .collect()
    }
}

pub(crate) type TimingsPriv = HashMap<SpanGroupPriv, Timing>;

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    callsite_info_path: CallsiteInfoPath,
    props_path: PropsPath,
    created_at: Instant,
}

//=================
// LatencyTraceCfg

pub(crate) type SpanGrouper =
    Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>;

pub(crate) struct LatencyTraceCfg {
    pub(crate) span_grouper: SpanGrouper,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

impl LatencyTraceCfg {
    /// Used to accumulate results on [`Control`].
    fn op(&self) -> impl Fn(TimingsPriv, &mut TimingsPriv, &ThreadId) + Send + Sync + 'static {
        let hist_high = self.hist_high;
        let hist_sigfig = self.hist_sigfig;
        move |timings: TimingsPriv, acc: &mut TimingsPriv, tid: &ThreadId| {
            log::debug!("executing `op` for {:?}", tid);
            for (k, v) in timings {
                let timing = acc
                    .entry(k)
                    .or_insert_with(|| new_timing(hist_high, hist_sigfig));
                timing.add(v).unwrap();
            }
        }
    }
}

//=================
// LatencyTracePriv

/// Provides access to [Timings] containing the latencies collected for different span groups.
#[derive(Clone)]
pub(crate) struct LatencyTracePriv {
    pub(crate) control: Control<TimingsPriv, TimingsPriv>,
    span_grouper: SpanGrouper,
    hist_high: u64,
    hist_sigfig: u8,
}

impl LatencyTracePriv {
    pub(crate) fn new(config: LatencyTraceCfg) -> LatencyTracePriv {
        LatencyTracePriv {
            control: Control::new(TimingsPriv::new(), config.op()),
            span_grouper: config.span_grouper,
            hist_high: config.hist_high,
            hist_sigfig: config.hist_sigfig,
        }
    }

    fn update_timings(&self, span_group_priv: &SpanGroupPriv, f: impl Fn(&mut Timing)) {
        self.control.with_tl_mut(&LOCAL_INFO, |timings| {
            let timing = {
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
                        new_timing(self.hist_high, self.hist_sigfig),
                    );
                    timings.get_mut(span_group_priv).unwrap()
                }
            };

            f(timing);

            log::trace!(
                "exiting `update_timings` for {:?} on {:?}",
                span_group_priv,
                thread::current().id()
            );
        });
    }

    /// Transforms a SpanGroupPriv to a SpanGroup and adds it to `sgp_to_sg`.
    fn grow_sgp_to_sg(sgp: &SpanGroupPriv, sgp_to_sg: &mut HashMap<SpanGroupPriv, SpanGroup>) {
        let parent_sgp = sgp.parent();
        let parent_id: Option<Arc<str>> = parent_sgp
            .iter()
            .map(|parent_sgp| match sgp_to_sg.get(parent_sgp) {
                Some(sg) => sg.id.clone(),
                None => {
                    Self::grow_sgp_to_sg(parent_sgp, sgp_to_sg);
                    sgp_to_sg.get(parent_sgp).unwrap().id.clone()
                }
            })
            .next();

        let callsite = sgp.callsite_info_path.last().unwrap().clone();
        let props = sgp.props_path.last().unwrap().clone();

        let mut hasher = Sha256::new();
        if let Some(parent_id) = parent_id.clone() {
            hasher.update(parent_id.as_ref());
        }
        hasher.update(format!("{:?}", callsite));
        hasher.update(format!("{:?}", props));
        let hash = hasher.finalize();
        let id = Base64::encode_string(&hash[0..12]);

        let sg = SpanGroup {
            callsite,
            props,
            depth: sgp.callsite_info_path.len(),
            id: id.into(),
            parent_id,
        };
        sgp_to_sg.insert(sgp.clone(), sg);
    }

    /// Generates the publicly accessible [`Timings`] in post-processing after all thread-local
    /// data has been accumulated.
    pub(crate) fn generate_timings(&self, tp: TimingsPriv) -> Timings {
        let mut sgp_to_sg: HashMap<SpanGroupPriv, SpanGroup> = HashMap::new();
        for sgp in tp.keys() {
            Self::grow_sgp_to_sg(sgp, &mut sgp_to_sg);
        }

        let mut timings: Timings = tp
            .into_iter()
            .map(|(sgp, timing)| (sgp_to_sg.remove(&sgp).unwrap(), timing))
            .collect();

        for sg in sgp_to_sg.into_values() {
            timings.insert(sg, new_timing(self.hist_high, self.hist_sigfig));
        }

        timings
    }

    /// This is exposed separately from [Self::generate_timings] to isolate the code that holds the control lock.
    /// This is useful in the implementation of `PausableTrace` in the `latency-trace` crate.
    pub(crate) fn take_latencies_priv(
        &self,
        lock: &mut ControlLock<'_, TimingsPriv>,
    ) -> TimingsPriv {
        self.control.take_acc(lock, TimingsPriv::new())
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

        let meta = span.metadata();
        let callsite_info = CallsiteInfo {
            name: span.name(),
            code_line: format!("{}:{}", meta.file().unwrap(), meta.line().unwrap()),
        };
        let props = (self.span_grouper)(attrs);
        let (callsite_info_path, props_path) = match parent_span {
            None => (vec![Arc::new(callsite_info)], vec![Arc::new(props)]),
            Some(parent_span) => {
                let ext = parent_span.extensions();
                let pst = ext.get::<SpanTiming>().unwrap();
                let mut callsite_info_path = pst.callsite_info_path.as_ref().clone();
                callsite_info_path.push(callsite_info.into());
                let mut props_path = pst.props_path.as_ref().clone();
                props_path.push(Arc::new(props));
                (callsite_info_path, props_path)
            }
        };

        span.extensions_mut().insert(SpanTiming {
            callsite_info_path: callsite_info_path.into(),
            props_path: props_path.into(),
            created_at: Instant::now(),
        });

        log::trace!("`on_new_span` end: name={}, id={:?}", span.name(), id);
    }

    // No need for fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {

    // No need for fn on_exit(&self, id: &Id, ctx: Context<'_, S>)

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).unwrap();
        log::trace!("`on_close` start: name={}, id={:?}", span.name(), id);

        let ext = span.extensions();
        let span_timing = ext.get::<SpanTiming>().unwrap();

        let span_group_priv = SpanGroupPriv {
            callsite_info_path: span_timing.callsite_info_path.clone(),
            props_path: span_timing.props_path.clone(),
        };

        self.update_timings(&span_group_priv, |timing| {
            timing
                .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .unwrap();
        });

        log::trace!(
            "`on_close` completed call to update_timings: name={}, id={:?}",
            span.name(),
            id
        );

        log::trace!("`on_close` end: name={}, id={:?}", span.name(), id);
    }
}

//=================
// Thread-locals

thread_local! {
    static LOCAL_INFO: Holder<TimingsPriv, TimingsPriv> = Holder::new(TimingsPriv::new);
}
