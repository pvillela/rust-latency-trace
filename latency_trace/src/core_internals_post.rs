//! Post-processing of timing information collected by [`crate::core_internals_pre`] to produce
//! information that is convenient to display.

use crate::{
    core_internals_pre::{
        new_timing, op_r, AccRawTrace, CallsiteInfo, LatencyTracePriv, Props, RawTrace,
        SpanGroupPriv, Timing,
    },
    summary_stats, SummaryStats, Wrapper,
};
use base64ct::{Base64, Encoding};
use hdrhistogram::Histogram;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    hash::Hash,
    sync::Arc,
};

pub type CallsiteInfoPath = Vec<Arc<CallsiteInfo>>;

//=================
// SpanGroup

/// Represents a set of [tracing::Span]s for which latency information should be collected as a group. It is
/// the unit of latency information collection.
///
/// Span definitions are created in the code using macros and functions from the Rust [tracing](https://crates.io/crates/tracing) library which define span ***callsite***s, i.e., the places in the code where spans are defined. As the code is executed, a span definition in the code may be executed multiple times -- each such execution is a span instance. Span instances arising from the same span definition are grouped into [`SpanGroup`]s for latency information collection. Latencies are collected using [Histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html)s from the [hdrhistogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/) library.
///
/// The grouping of spans for latency collection is not exactly based on the span definitions in the code. Spans at runtime are structured as a set of [span trees](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships) that correspond to the nesting of spans from code execution paths. The grouping of runtime spans for latency collection should respect the runtime parent-child relationships among spans.
///
/// Thus, [`SpanGroup`]s form a forest of trees where some pairs of span groups have a parent-child relationship, corresponding to the parent-child relationships of the spans associated with the span groups. This means that if `SpanGroup A` is the parent of `SpanGroup B` then, for each span that was assigned to group `B`, its parent span was assigned to group `A`.
///
/// The coarsest-grained grouping of spans is characterized by a ***callsite path*** -- a callsite and the (possibly empty) list of its ancestor callsites based on the different runtime execution paths (see [Span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships)). This is the default `SpanGroup` definition. Finer-grained groupings of spans can differentiate groups of spans with the same callsite path by taking into account values computed at runtime from the spans' runtime [Attributes](https://docs.rs/tracing/0.1.37/tracing/span/struct.Attributes.html).
///
/// This struct holds the following information:
/// - the name [`name`](Self::name) of the span definition that applies to all the spas in the span group
/// - an [`id`](Self::id) that, together with its `name`, uniquely identifies the span group
/// - a [`props`](Self::props) field that contains the list of name-value pairs (which may be empty) which is common to all the spans in the group
/// - a [`code_line`](Self::code_line) field that contains the file name and line number where all the spans in the group were defined *or*,
///   in case debug information is not available, the corresponding [`tracing::callsite::Identifier`].
/// - a [`parent_id`](Self::parent_id) that is the `id` field of the parent span group, if any.
/// - its [`depth`](Self::depth) that is the number of ancestor span groups this span group has
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Clone)]
pub struct SpanGroup {
    name: &'static str,
    id: Arc<str>,
    code_line: Arc<str>,
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

    /// Returns the number of ancestor span groups this span group has.
    pub fn depth(&self) -> usize {
        self.depth
    }
}

/// Intermediate form of [`SpanGroup`] used in post-processing when transforming from [`SpanGroupPriv`]
/// to [`SpanGroup`].
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SpanGroupTemp {
    span_group_priv: SpanGroupPriv,
    callsite_info_priv_path: CallsiteInfoPath,
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
            callsite_info_priv_path,
        })
    }
}

//=================
// Timings

/// [`Wrapper`] of [`BTreeMap`]`<K, `[`Timing`]`>`; inherits all [`BTreeMap`] methods.
pub type TimingsView<K> = Wrapper<BTreeMap<K, Timing>>;

impl<K> TimingsView<K> {
    /// Combines histogram values according to sets of keys that yield the same value when `f`
    /// is applied.
    pub fn aggregate<G>(&self, f: impl Fn(&K) -> G) -> TimingsView<G>
    where
        G: Ord,
    {
        let mut res: BTreeMap<G, Histogram<u64>> = BTreeMap::new();
        for (k, v) in self.iter() {
            // Construct aggregation map.
            let g = f(k);
            let hist = match res.get_mut(&g) {
                Some(hist) => hist,
                None => {
                    res.insert(g, Histogram::new_from(v));
                    res.get_mut(&f(k))
                        .expect("key `g == f(k)` was just inserted in `res`")
                }
            };
            hist.add(v)
                .expect("should not happen given histogram construction");
        }
        res.into()
    }

    /// Combines the histograms of `self` with those of another [`TimingsView`].
    pub fn add(&mut self, mut other: TimingsView<K>)
    where
        K: Ord,
    {
        // Combine into self the values in other that have keys in self.
        for (k, h) in self.iter_mut() {
            let other_h = other.remove(k);
            if let Some(other_h) = other_h {
                h.add(other_h)
                    .expect("should not happen given histogram construction");
            }
        }

        // Insert into self the entries in other that don't have keys in self.
        for (k, h) in other.0.into_iter() {
            self.insert(k, h);
        }
    }

    /// Produces a map whose values are the [`SummaryStats`] of `self`'s histogram values.
    pub fn summary_stats(&self) -> Wrapper<BTreeMap<K, SummaryStats>>
    where
        K: Ord + Clone,
    {
        self.map_values(summary_stats)
    }
}

/// Mapping of [`SpanGroup`]s to the [`Timing`] information recorded for them; inherits all [`BTreeMap`] methods.
pub type Timings = TimingsView<SpanGroup>;

impl Timings {
    /// Checks whether an aggregation function `f` used in [`Self::aggregate`] is consistent according to the following
    /// definition:
    /// - the values resulting from applying `f` to span groups are called ***aggregate key***s
    /// - the sets of span groups corresponding to each *aggregate key* are called ***aggregates***.
    ///
    /// An aggregation function is consistent if and only if, for each *aggregate*, all the span groups in the
    /// *aggregate* have the same callsite.
    pub fn aggregator_is_consistent<G>(&self, f: impl Fn(&SpanGroup) -> G) -> bool
    where
        G: Ord,
    {
        let mut aggregates: BTreeMap<G, Arc<str>> = BTreeMap::new();
        let mut is_consistent = true;
        for k in self.keys() {
            let g = f(k);
            if is_consistent {
                is_consistent = match aggregates.get(&g) {
                    Some(code_line) => code_line.as_ref() == k.code_line(),
                    None => {
                        aggregates.insert(g, k.code_line.clone());
                        true
                    }
                };
            }
        }
        is_consistent
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
                let parent = sg.parent_id().map(|pid| {
                    id_to_sg
                        .get(pid)
                        .expect("`id_to_sg` must have key `pid` by construction")
                        .clone()
                });
                (sg.clone(), parent)
            })
            .collect()
    }
}

//=================
// Post-processing

/// Intermediate form of latency information collected for span groups, used during post-processing while
/// transforming [`SpanGroupPriv`] to [`SpanGroup`].
type TimingsTemp = HashMap<SpanGroupTemp, Timing>;

/// Part of post-processing.
/// Reduces acc to TimingsPriv.
fn reduce_acc_to_raw_trace(acc: AccRawTrace) -> RawTrace {
    log::trace!("entering `reduce_acc_to_timings_priv`");
    acc.into_iter().fold(RawTrace::new(), op_r)
}

/// Part of post-processing.
/// Moves callsite info in [`RawTrace`] values into the keys in [TimingsTemp].
fn move_callsite_info_to_key(raw_trace: RawTrace) -> TimingsTemp {
    log::trace!("entering `move_callsite_info_to_key`");
    let RawTrace {
        timings,
        callsite_infos,
    } = raw_trace;
    timings
        .into_iter()
        .map(|(span_group_priv, hist)| {
            let callsite_info_priv_path: CallsiteInfoPath = span_group_priv
                .callsite_id_path
                .iter()
                .map(|id| {
                    callsite_infos
                        .get(id)
                        .expect("`callsite_infos` must have key `id` by construction")
                        .clone()
                        .into()
                })
                .collect();
            let sgt = SpanGroupTemp {
                span_group_priv,
                callsite_info_priv_path,
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
                grow_sgt_to_sg(parent_sgp, sgt_to_sg);
                sgt_to_sg
                    .get(parent_sgp)
                    .expect("key `parent_sgp` must exist in `sgt_to_sg` by construction")
                    .id
                    .clone()
            }
        })
        .next();

    let callsite_info = sgt
        .callsite_info_priv_path
        .last()
        .expect("sgt.callsite_info_priv_path can't be empty by construction");

    let code_line = callsite_info
        .file
        .clone()
        .zip(callsite_info.line)
        .map(|(file, line)| format!("{}:{}", file, line))
        .unwrap_or_else(|| format!("{:?}", callsite_info.callsite_id));

    let props = sgt
        .span_group_priv
        .props_path
        .last()
        .expect("sgt.span_group_priv.props_path can't be empty by construction")
        .clone();

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
/// Transforms TimingsTemp and sgt_to_sg into Timings.
fn timings_from_timings_temp_and_spt_to_sg(
    ltp: &LatencyTracePriv,
    timings_temp: TimingsTemp,
    mut sgt_to_sg: HashMap<SpanGroupTemp, SpanGroup>,
) -> Timings {
    // Transform `timings_temp` into `timings` by changing keys from sgt to sg, and remove those keys from `sgt_to_sg`.
    let mut timings: Timings = timings_temp
        .into_iter()
        .map(|(sgt, timing)| {
            (
                sgt_to_sg
                    .remove(&sgt)
                    .expect("impossible: sgt key not found in sgt_to_sg"),
                timing,
            )
        })
        .collect::<BTreeMap<SpanGroup, Timing>>()
        .into();

    // Add entries with empty histograms for span groups that are not already keys in `timings`.
    for sg in sgt_to_sg.into_values() {
        timings.insert(sg, new_timing(ltp.hist_high, ltp.hist_sigfig));
    }

    timings
}

/// Post-processing orchestration of the above functions.
/// Generates the publicly accessible [`Timings`] in post-processing after all thread-local
/// data has been accumulated.
pub(crate) fn report_timings(ltp: &LatencyTracePriv, acc: AccRawTrace) -> Timings {
    log::trace!("entering `report_timings`");
    // Reduce acc to RawTrace
    let raw_trace: RawTrace = reduce_acc_to_raw_trace(acc);

    // Transform RawTrace into TimingsTemp and sgt_to_sg.
    let timings_temp = move_callsite_info_to_key(raw_trace);
    let mut sgt_to_sg: HashMap<SpanGroupTemp, SpanGroup> = HashMap::new();
    for sgt in timings_temp.keys() {
        grow_sgt_to_sg(sgt, &mut sgt_to_sg);
    }

    // Transform TimingsTemp and sgt_to_sg into Timings.
    timings_from_timings_temp_and_spt_to_sg(ltp, timings_temp, sgt_to_sg)
}
