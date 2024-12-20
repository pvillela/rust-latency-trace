//! Publicly exported core [`LatencyTrace`]-related types and methods.

use std::{collections::BTreeMap, sync::Arc, thread};

use hdrhistogram::Histogram;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::{
    lt_collect_g::LatencyTraceG,
    summary_stats,
    tlc_param::{Either, Joined, Probed},
    SummaryStats, Wrapper,
};
pub use crate::{
    lt_collect_g::{LatencyTraceCfg, Timing},
    lt_refine_g::{SpanGroup, Timings, TimingsView},
    lt_report_g::ActivationError,
    probed_trace::ProbedTrace,
};

//==============
// Exported aliases

#[doc(hidden)]
/// Used for benchmarking purposes only
pub type LatencyTraceJ = LatencyTraceG<Joined>;

#[doc(hidden)]
/// Used for benchmarking purposes only
pub type LatencyTraceE = LatencyTraceG<Either>;

impl LatencyTraceE {
    pub fn select_probed() {
        Either::select_probed();
    }

    pub fn select_joined() {
        Either::select_joined()
    }
}

//==============
// pub impl for LatencyTraceCfg

impl LatencyTraceCfg {
    /// Creates a new [`LatencyTraceCfg`] the same as `self` but with the given `hist_high`
    /// (see [hdrhistogram::Histogram::high]).
    pub fn with_hist_high(&self, hist_high: u64) -> Self {
        LatencyTraceCfg {
            span_grouper: self.span_grouper.clone(),
            hist_high,
            hist_sigfig: self.hist_sigfig,
        }
    }

    /// Creates a new [`LatencyTraceCfg`] the same as `self` but with the given `hist_sigfig`
    /// (see [hdrhistogram::Histogram::sigfig]).
    pub fn with_hist_sigfig(&self, hist_sigfig: u8) -> Self {
        LatencyTraceCfg {
            span_grouper: self.span_grouper.clone(),
            hist_high: self.hist_high,
            hist_sigfig,
        }
    }

    /// Creates a new [`LatencyTraceCfg`] the same as `self` but with the given `span_grouper`.
    pub fn with_span_grouper(
        &self,
        span_grouper: impl Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static,
    ) -> Self {
        LatencyTraceCfg {
            span_grouper: Arc::new(span_grouper),
            hist_high: self.hist_high,
            hist_sigfig: self.hist_sigfig,
        }
    }
}

//==============
// pub impl for LatencyTrace

/// Core type supporting latency mesurements.
///
/// An implementation of [`tracing_subscriber::Layer`] that provides access to the latencies collected
/// for different span groups.
///
/// If directly instantiated as a [`tracing::Subscriber`] using the `activated*` methods,
/// there should be a single instance (except for its clones) of [`LatencyTrace`] in a process. That instance is set
/// (by method [`Self::activated`] or [`Self::activated_default`])
/// as the global default [`tracing::Subscriber`], of which there can be only one and it can't be changed once
/// it is set.
#[derive(Clone)]
pub struct LatencyTrace(pub(crate) LatencyTraceG<Probed>);

impl LatencyTrace {
    /// Constructs `Self` with the given configuration. Can be used to construct an instance for use as a [`Layer`].
    pub fn new(config: LatencyTraceCfg) -> Self {
        Self(LatencyTraceG::new(config))
    }

    /// Returns the active instance of `Self` if it exists. An active instance is an instance that is registered as
    /// the global default [`tracing::Subscriber`].
    pub fn active() -> Option<Self> {
        Some(Self(LatencyTraceG::active()?))
    }

    /// Convenience method that creates a layered [`tracing::Subscriber`] with a [`LatencyTrace`] as the single layer.
    /// Returns the active instance of `Self` if it exists or activates a new instance with the given `config` otherwise.
    /// Activation entails setting the global default [`tracing::Subscriber`], of which there can be only one and it can't
    /// be changed once it is set.
    ///
    /// If a [`LatencyTrace`] has been previously activated in the same process, the `config` passed to this
    /// function will be ignored and the current active [`LatencyTrace`] will be returned.
    ///
    /// # Errors
    /// - [`ActivationError::HistogramConfigError`] if the `config`'s `hist_high` and `hist_sigfig` would cause
    ///   [`hdrhistogram::Histogram::new_with_bounds`]`(1, hist_high, hist_sigfig)` to fail.
    /// - [`ActivationError::TracingSubscriberInitError`] if a global [`tracing::Subscriber`] is already set and its
    ///   type is not the same as `Self`.
    pub fn activated(config: LatencyTraceCfg) -> Result<Self, ActivationError> {
        Ok(Self(LatencyTraceG::activated(config)?))
    }

    /// Convenience method that creates a layered [`tracing::Subscriber`] with a [`LatencyTrace`] as the single layer.
    /// Returns the active instance of `Self` if it exists or activates a new instance with the default configuration otherwise.
    /// Activation entails setting the global default [`tracing::Subscriber`], of which there can be only one and it can't
    /// be changed once it is set.
    ///
    /// If a [`LatencyTrace`] has been previously activated in the same process, the default configuration
    /// will be ignored and the current active [`LatencyTrace`] will be returned.
    ///
    /// # Errors
    /// - [`ActivationError::TracingSubscriberInitError`] if a global [`tracing::Subscriber`] is already set and its
    ///   type is not the same as `Self`.
    pub fn activated_default() -> Result<Self, ActivationError> {
        Ok(Self(LatencyTraceG::activated_default()?))
    }

    /// Executes the instrumented function `f` and, after `f` completes, returns the observed latencies.
    pub fn measure_latencies(&self, f: impl FnOnce()) -> Timings {
        self.0.measure_latencies(f)
    }

    /// Executes the instrumented function `f`, returning a [`ProbedTrace`] that allows partial latencies to be
    /// reported before `f` completes.
    pub fn measure_latencies_probed(
        self,
        f: impl FnOnce() + Send + 'static,
    ) -> Result<ProbedTrace, ActivationError> {
        let pt = ProbedTrace::new(self);
        let jh = thread::spawn(f);
        pt.set_join_handle(jh);
        Ok(pt)
    }
}

impl Default for LatencyTrace {
    /// Constructs `Self` with default configuration. Can be used to construct an instance for use as a [`Layer`].
    fn default() -> Self {
        Self::new(LatencyTraceCfg::default())
    }
}
impl<S> Layer<S> for LatencyTrace
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        self.0.on_new_span(attrs, id, ctx);
    }

    // No need for fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {

    // No need for fn on_exit(&self, id: &Id, ctx: Context<'_, S>)

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        self.0.on_close(id, ctx);
    }
}

//==============
// pub impl for SpanGroup

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

//==============
// pub impl for TimingsView

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

//==============
// pub impl for Timings

impl Timings {
    /// Checks whether an aggregation function `f` used in [`Self::aggregate`] is consistent according to the following
    /// definition:
    /// - the values resulting from applying `f` to span groups are called ***aggregate key***s
    /// - the sets of span groups corresponding to each *aggregate key* are called ***aggregates***.
    /// - an aggregation function is consistent if and only if, for each *aggregate*, all the span groups in the
    ///   *aggregate* have the same callsite.
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

    /// Returns a map that associates each [`SpanGroup`] to its parent.
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
