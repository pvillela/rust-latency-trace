use crate::Wrapper;
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

    pub fn depth(&self) -> usize {
        self.depth
    }
}

//=================
// Timings

/// [`Wrapper`] of [`BTreeMap`]`<K, `[`Timing`]`>`; inherits all [`BTreeMap`] methods.
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

/// Mapping of [`SpanGroup`]s to the [`Timing`] information recorded for them; inherits all [`BTreeMap`] methods.
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
        let mut aggregates: BTreeMap<G, Arc<str>> = BTreeMap::new();
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
