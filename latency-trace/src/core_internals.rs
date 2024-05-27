//! Core library implementation.

use base64ct::{Base64, Encoding};
use hdrhistogram::Histogram;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use thread_local_collect::tlm::probed::{Control, Holder};
use tracing::{callsite::Identifier, span::Attributes, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::Wrapper;

//=================
// Callsite

/// Provides information about where the tracing span was defined.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct CallsiteInfoPriv {
    callsite_id: Identifier,
    name: &'static str,
    file: Option<String>,
    line: Option<u32>,
}

//=================
// Paths

// Types used in span groups or to support data collection.

type CallsiteIdPath = Arc<Vec<Identifier>>;
type CallsiteInfoPrivPath = Arc<Vec<Arc<CallsiteInfoPriv>>>;
type Props = Vec<(String, String)>;
type PropsPath = Arc<Vec<Arc<Props>>>;

//=================
// SpanGroup

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
/// - its [`name`](Self::name)
/// - an [`id`](Self::id) that, together with its `name`, uniquely identifies the span group
/// - a [`props`](Self::props) field that contains the span group's list of name-value pairs (which may be empty)
/// - a [`code_line`](Self::code_line) field that contains the file name and line number where the span was defined *or*,
///   in case debug information is not available, the callsite [`Identifier`].
/// - a [`parent_id`](Self::parent_id) that is the `id` field of the parent span group, if any.
/// - its [`depth`](Self::depth) that is the number of ancestor span groups this span group has
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    name: &'static str,
    id: Arc<str>,
    code_line: Arc<String>,
    props: Arc<Props>,
    parent_id: Option<Arc<str>>,
    depth: usize,
}

impl SpanGroup {
    /// Returns the span group's name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the span group's ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the span group's file name and code line.
    pub fn code_line(&self) -> &str {
        &self.code_line
    }

    /// Returns the span group's properties list.
    ///
    /// This list can be empty as is the case with the default span grouper.
    pub fn props(&self) -> &[(String, String)] {
        &self.props
    }

    /// Returns the ID of the span group's parent.
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.iter().map(|x| x.as_ref()).next()
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

/// Private form of [`SpanGroup`] used during trace collection, more efficient than [`SpanGroup`] for trace
/// data collection.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct SpanGroupPriv {
    /// Callsite ID of the span group preceded by the callsite IDs of its ancestors.
    callsite_id_path: CallsiteIdPath,

    /// Properties of the span group preceded by the properties of its ancestors.
    props_path: PropsPath,
}

impl SpanGroupPriv {
    fn parent(&self) -> Option<Self> {
        let len = self.callsite_id_path.len();
        if len == 1 {
            return None;
        }
        Some(SpanGroupPriv {
            callsite_id_path: Arc::new(self.callsite_id_path[0..len - 1].into()),
            props_path: Arc::new(self.props_path[0..len - 1].into()),
        })
    }
}

/// Intermediate form of [`SpanGroup`] used in post-processing when transforming between [`SpanGroupPriv`]
/// and [`SpanGroup`].
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupTemp {
    span_group_priv: SpanGroupPriv,
    callsite_info_priv_path: CallsiteInfoPrivPath,
}

impl SpanGroupTemp {
    fn parent(&self) -> Option<Self> {
        let parent_sgp = match self.span_group_priv.parent() {
            None => return None,
            Some(sgp) => sgp,
        };
        let len = self.span_group_priv.callsite_id_path.len();
        let callsite_info_priv_path = self.callsite_info_priv_path[0..len - 1].to_vec();
        Some(SpanGroupTemp {
            span_group_priv: parent_sgp,
            callsite_info_priv_path: callsite_info_priv_path.into(),
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

#[derive(Debug, Clone)]
pub(crate) struct TimingPriv {
    hist: Timing,
    callsite_info_priv_path: CallsiteInfoPrivPath,
}

impl TimingPriv {
    fn new(hist_high: u64, hist_sigfig: u8, callsite_info_priv_path: CallsiteInfoPrivPath) -> Self {
        let hist = new_timing(hist_high, hist_sigfig);
        Self {
            hist,
            callsite_info_priv_path,
        }
    }
}

//=================
// Timings

/// [`Wrapper`] of [`BTreeMap`]`<K, `[`Timing`]`>`.
pub type TimingsView<K> = Wrapper<BTreeMap<K, Timing>>;

impl<K> TimingsView<K> {
    pub fn add(&mut self, mut other: TimingsView<K>)
    where
        K: Ord,
    {
        // Combine into self the values in other that have keys in self.
        for (k, h) in self.iter_mut() {
            let other_h = other.remove(k);
            if let Some(other_h) = other_h {
                h.add(other_h).unwrap();
            }
        }

        // Insert into self the entries in other that don't have keys in self.
        for (k, h) in other.0.into_iter() {
            self.insert(k, h);
        }
    }
}

/// Mapping of [`SpanGroup`]s to the [`Timing`] information recorded for them.
pub type Timings = TimingsView<SpanGroup>;

impl Timings {
    /// Combines histograms of span groups according to sets of span groups that yield the same value when `f`
    /// is applied. The values resulting from applying `f` to span groups are called ***aggregate key***s and
    /// the sets of span groups corresponding to each *aggregate key* are called ***aggregates***.
    ///
    /// An aggregation is consistent if and only if, for each *aggregate*, all the span groups in the *aggregate*
    /// have the same callsite.
    ///
    /// This function returns a pair with the following components:
    /// - a [BTreeMap] that associates each *aggregate key* to its aggregated histogram;
    /// - a boolean that is `true` if the aggregation is consistent, `false` otherwise.
    pub fn aggregate<G>(&self, f: impl Fn(&SpanGroup) -> G) -> (BTreeMap<G, Timing>, bool)
    where
        G: Ord + Clone,
    {
        let mut res: BTreeMap<G, Histogram<u64>> = BTreeMap::new();
        let mut aggregates: BTreeMap<G, Arc<String>> = BTreeMap::new();
        let mut aggregates_are_consistent = true;
        for (k, v) in self.iter() {
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
                    Some(code_line) => code_line.as_ref() == k.code_line(),
                    None => {
                        aggregates.insert(g.clone(), k.code_line.clone());
                        true
                    }
                };
            }
        }

        (res, aggregates_are_consistent)
    }

    /// Returns a map from span group ID to [`SpanGroup`].
    fn id_to_span_group(&self) -> BTreeMap<String, SpanGroup> {
        self.keys()
            .map(|k| (k.id().to_owned(), k.clone()))
            .collect()
    }

    /// Returns a map from [`SpanGroup`] to its parent.
    pub fn span_group_to_parent(&self) -> BTreeMap<SpanGroup, Option<SpanGroup>> {
        let id_to_sg = self.id_to_span_group();
        self.keys()
            .map(|sg| {
                let parent = sg.parent_id().map(|pid| id_to_sg.get(pid).unwrap().clone());
                (sg.clone(), parent)
            })
            .collect()
    }
}

/// Type of latency information internally collected for span groups. The key is [SpanGroupPriv], which is as
/// light as possible to minimize processing overhead when accessing the map. Therefore, part of the information
/// required to produce the ultimate results is kept in the map's values as the `callsite_info_priv_path` field
/// in [TimingsPriv].
type TimingsPriv = HashMap<SpanGroupPriv, TimingPriv>;

/// Intermediate form of latency information collected for span groups, used during post-processing while
/// transforming [`SpanGroupPriv`] to [`SpanGroup`].
type TimingsTemp = HashMap<SpanGroupTemp, Timing>;

/// Type of accumulator of thread-local values, prior to transforming the collected information to a [`Timings`].
/// Used to minimize the time holding the control lock during post-processing.
type AccTimings = Vec<(ThreadId, HashMap<SpanGroupPriv, TimingPriv>)>;

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    callsite_info_priv_path: CallsiteInfoPrivPath,
    props_path: PropsPath,
    created_at: Instant,
}

//=================
// SpanGrouper

/// Internal type of span groupers.
pub(crate) type SpanGrouper =
    Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>;

//=================
// LatencyTraceCfg

/// Configuration information used by both [`LatencyTracePriv`] and [`LatencyTrace`](super::LatencyTrace).
pub(crate) struct LatencyTraceCfg {
    pub(crate) span_grouper: SpanGrouper,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

impl LatencyTraceCfg {
    /// Used to accumulate results on [`Control`].
    fn op(&self) -> impl Fn(TimingsPriv, &mut AccTimings, ThreadId) + Send + Sync + 'static {
        // let hist_high = self.hist_high;
        // let hist_sigfig = self.hist_sigfig;
        move |timings: TimingsPriv, acc: &mut AccTimings, tid: ThreadId| {
            log::debug!("executing `op` for {:?}", tid);
            // for (k, v) in timings {
            //     let timing_priv = acc
            //         .entry(k)
            //         .or_insert_with(|| new_timing(hist_high, hist_sigfig));
            //     timing_priv.add(v.hist).unwrap();
            // }
            acc.push((tid, timings));
        }
    }
}

//=================
// LatencyTracePriv

/// Implements [Layer] and provides access to [Timings] containing the latencies collected for different span groups.
/// This is the central struct in this framework's implementation.
#[derive(Clone)]
pub(crate) struct LatencyTracePriv {
    pub(crate) control: Control<TimingsPriv, AccTimings>,
    span_grouper: SpanGrouper,
    hist_high: u64,
    hist_sigfig: u8,
}

impl LatencyTracePriv {
    pub(crate) fn new(config: LatencyTraceCfg) -> LatencyTracePriv {
        LatencyTracePriv {
            control: Control::new(
                &LOCAL_INFO,
                AccTimings::new(),
                TimingsPriv::new,
                config.op(),
            ),
            span_grouper: config.span_grouper,
            hist_high: config.hist_high,
            hist_sigfig: config.hist_sigfig,
        }
    }

    /// Updates timings for the given span group. Called by [Layer] impl.
    fn update_timings(
        &self,
        span_group_priv: &SpanGroupPriv,
        callsite_info_priv_path: &CallsiteInfoPrivPath,
        f: impl Fn(&mut TimingPriv),
    ) {
        self.control.with_data_mut(|timings| {
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
                        TimingPriv::new(
                            self.hist_high,
                            self.hist_sigfig,
                            callsite_info_priv_path.clone(),
                        ),
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

    /// Extracts the accumulated timings.
    pub(crate) fn take_acc_timings(&self) -> AccTimings {
        log::trace!("entering `take_acc_timings`");
        self.control.take_tls();
        self.control.take_acc(AccTimings::new())
    }

    pub(crate) fn probe_acc_timings(&self) -> AccTimings {
        log::trace!("entering `take_acc_timings`");
        self.control.probe_tls()
    }

    /// Part of post-processing.
    /// Moves callsite info in [TimingsPriv] values into the keys in [TimingsTemp].
    fn move_callsite_info_to_key(timings_priv: TimingsPriv) -> TimingsTemp {
        log::trace!("entering `move_callsite_info_to_key`");
        timings_priv
            .into_iter()
            .map(|(k, v)| {
                let callsite_priv = v.callsite_info_priv_path;
                let hist = v.hist;
                let sgt = SpanGroupTemp {
                    span_group_priv: k,
                    callsite_info_priv_path: callsite_priv,
                };
                (sgt, hist)
            })
            .collect()
    }

    /// Part of post-processing.
    /// Transforms a [SpanGroupTemp] into a [SpanGroup] and adds it to `sgt_to_sg`.
    ///
    /// This function serves two purposes:
    /// - Generates span groups that have not yet received any timing information and therefore do not
    ///   appear as keys in the thread-local TimingsPriv maps. This can happen for parent span groups
    ///   when using [super::ProbedTrace].
    /// - Generates the span group IDs, which are inherently recursive as a span group's ID is a hash that
    ///   depends on its parent's ID.
    fn grow_sgt_to_sg(sgt: &SpanGroupTemp, sgt_to_sg: &mut HashMap<SpanGroupTemp, SpanGroup>) {
        log::trace!("entering `grow_sgt_to_sg`");
        let parent_sgt = sgt.parent();
        let parent_id: Option<Arc<str>> = parent_sgt
            .iter()
            .map(|parent_sgp| match sgt_to_sg.get(parent_sgp) {
                Some(sg) => sg.id.clone(),
                None => {
                    Self::grow_sgt_to_sg(parent_sgp, sgt_to_sg);
                    sgt_to_sg.get(parent_sgp).unwrap().id.clone()
                }
            })
            .next();

        let callsite_info = sgt.callsite_info_priv_path.last().unwrap();

        let code_line = callsite_info
            .file
            .clone()
            .zip(callsite_info.line)
            .map(|(file, line)| format!("{}:{}", file, line))
            .unwrap_or_else(|| format!("{:?}", callsite_info.callsite_id));

        let props = sgt.span_group_priv.props_path.last().unwrap().clone();

        let mut hasher = Sha256::new();
        if let Some(parent_id) = parent_id.clone() {
            hasher.update(parent_id.as_ref());
        }
        hasher.update(callsite_info.name);
        hasher.update([0_u8; 1]);
        hasher.update(code_line.clone());
        for (k, v) in props.iter() {
            hasher.update([0_u8; 1]);
            hasher.update(k);
            hasher.update([0_u8; 1]);
            hasher.update(v);
        }
        let hash = hasher.finalize();
        let id = Base64::encode_string(&hash[0..8]);

        let sg = SpanGroup {
            name: callsite_info.name,
            id: id.into(),
            code_line: code_line.into(),
            props,
            parent_id,
            depth: sgt.callsite_info_priv_path.len(),
        };
        sgt_to_sg.insert(sgt.clone(), sg);
    }

    /// Part of post-processing.
    /// Reduces acc to TimingsPriv.
    fn reduce_acc_to_timings_priv(acc: AccTimings) -> TimingsPriv {
        log::trace!("entering `reduce_acc_to_timings_priv`");
        let mut timings_priv: TimingsPriv = TimingsPriv::new();
        for (tid, m) in acc.into_iter() {
            println!("{:?} -> {}", tid, m.len());
            for (k, v) in m {
                let tp = timings_priv.get_mut(&k);
                match tp {
                    Some(tp) => tp.hist.add(v.hist).unwrap(),
                    None => {
                        timings_priv.insert(k, v);
                    }
                }
            }
        }
        log::trace!("exiting `reduce_acc_to_timings_priv`");
        timings_priv
    }

    /// Post-processing orchestration of the above functions.
    /// Generates the publicly accessible [`Timings`] in post-processing after all thread-local
    /// data has been accumulated.
    pub(crate) fn report_timings(&self, acc: AccTimings) -> Timings {
        log::trace!("entering `report_timings`");
        // Reduces acc to TimingsPriv
        let timings_priv: TimingsPriv = Self::reduce_acc_to_timings_priv(acc);

        // Transform TimingsPriv into TimingsTemp and sgt_to_sg.
        let timings_temp = Self::move_callsite_info_to_key(timings_priv);
        let mut sgt_to_sg: HashMap<SpanGroupTemp, SpanGroup> = HashMap::new();
        for sgt in timings_temp.keys() {
            Self::grow_sgt_to_sg(sgt, &mut sgt_to_sg);
        }

        // Transform TimingsTemp and sgt_to_sg into Timings.
        let mut timings: Timings = timings_temp
            .into_iter()
            .map(|(sgt, timing)| (sgt_to_sg.remove(&sgt).unwrap(), timing))
            .collect::<BTreeMap<SpanGroup, Timing>>()
            .into();
        for sg in sgt_to_sg.into_values() {
            timings.insert(sg, new_timing(self.hist_high, self.hist_sigfig));
        }

        timings
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
        let callsite_info = CallsiteInfoPriv {
            name: span.name(),
            callsite_id: meta.callsite(),
            file: meta.file().map(|s| s.to_owned()),
            line: meta.line(),
        };
        let props = (self.span_grouper)(attrs);
        let (callsite_info_path, props_path) = match parent_span {
            None => (vec![Arc::new(callsite_info)], vec![Arc::new(props)]),
            Some(parent_span) => {
                let ext = parent_span.extensions();
                let pst = ext.get::<SpanTiming>().unwrap();
                let mut callsite_info_path = pst.callsite_info_priv_path.as_ref().clone();
                callsite_info_path.push(callsite_info.into());
                let mut props_path = pst.props_path.as_ref().clone();
                props_path.push(Arc::new(props));
                (callsite_info_path, props_path)
            }
        };

        span.extensions_mut().insert(SpanTiming {
            callsite_info_priv_path: callsite_info_path.into(),
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
            callsite_id_path: span_timing
                .callsite_info_priv_path
                .iter()
                .map(|x| x.callsite_id.clone())
                .collect::<Vec<_>>()
                .into(),
            props_path: span_timing.props_path.clone(),
        };

        self.update_timings(
            &span_group_priv,
            &span_timing.callsite_info_priv_path,
            |tp| {
                tp.hist
                    .record((Instant::now() - span_timing.created_at).as_micros() as u64)
                    .unwrap();
            },
        );

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
    // static LOCAL_INFO: Holder<TimingsPriv, AccTimings> = Holder::new(TimingsPriv::new);
    static LOCAL_INFO: Holder<TimingsPriv, AccTimings> = Holder::new();
}
