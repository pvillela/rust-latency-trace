//! Core [`LatencyTrace`]-related types, methods, and functions exported publicly.

use hdrhistogram::{CreationError, Histogram};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Debug, Display},
    future::Future,
    sync::Arc,
    thread,
};
use tracing::{span::Attributes, Dispatch};
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    util::{SubscriberInitExt, TryInitError},
    Registry,
};

use crate::{
    default_span_grouper, summary_stats,
    tlc_param::{Probed, TlcBase, TlcJoined, TlcParam, TlcProbed},
    SummaryStats, Wrapper,
};
use crate::{lt_collect_g::LatencyTraceG, probed_trace_g::ProbedTraceG};

pub use crate::{
    lt_collect_g::{LatencyTraceCfg, Timing},
    lt_refine_g::{SpanGroup, Timings, TimingsView},
};

pub type LatencyTrace = LatencyTraceG<Probed>;
pub type ProbedTrace = ProbedTraceG<Probed>;

//==============
// Errors

/// Error returned by [`LatencyTrace`] activation methods.
#[derive(Debug)]
pub enum ActivationError {
    HistogramConfigError,
    TracingSubscriberInitError,
}

impl Display for ActivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for ActivationError {}

impl From<CreationError> for ActivationError {
    fn from(_: CreationError) -> Self {
        Self::HistogramConfigError
    }
}

impl From<TryInitError> for ActivationError {
    fn from(_: TryInitError) -> Self {
        Self::TracingSubscriberInitError
    }
}

//==============
// pub impl for LatencyTraceCfg

impl Default for LatencyTraceCfg {
    /// Instantiates a default [LatencyTraceCfg]. The defaults are:
    /// - Grouping of spans using the [`default_span_grouper`], which simply groups by the span's
    /// callsite information, which distills the *tracing* framework's Callsite concept
    /// (see [Metadata and Callsite](https://docs.rs/tracing-core/0.1.31/tracing_core/)). This default can be
    /// modified by using the [`Self::with_span_grouper`] method.
    /// - `hist_high` of `20,000,000` (20 seconds). This default can be modified by using the [`Self::with_hist_high`] method.
    /// - `hist_sigfig` of 2. This default can be modified by using the [`Self::with_hist_sigfig`] method.
    ///
    /// See [hdrhistogram::Histogram::high] and [hdrhistogram::Histogram::sigfig] for an explanation of these histogram configuration parameters.
    ///
    /// Note that the histograms used here are auto-resizable, which means [`hdrhistogram::Histogram::high`] is
    /// automatically adjusted as needed (although resizing requires memory reallocation at runtime).
    fn default() -> Self {
        LatencyTraceCfg {
            span_grouper: Arc::new(default_span_grouper),
            hist_high: 20 * 1000 * 1000,
            hist_sigfig: 2,
        }
    }
}

impl LatencyTraceCfg {
    /// Validates that the configuration settings yield histograms that avoid all potential [hdrhistogram::Histogram] errors
    /// as our histograms are `u64`, have a `hist_low` of `1`, and are auto-resizable.
    fn validate_hist_high_sigfig(&self) -> Result<(), CreationError> {
        let _ = Timing::new_with_bounds(1, self.hist_high, self.hist_sigfig)?;
        Ok(())
    }

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

impl<P> LatencyTraceG<P>
where
    P: TlcParam + Clone + 'static,
    P::Control: TlcBase + Clone,
    Layered<LatencyTraceG<P>, Registry>: Into<Dispatch>,
{
    /// Returns the active instance of `Self` if it exists.
    pub fn active() -> Option<Self> {
        tracing::dispatcher::get_default(|disp| {
            let lt: &Self = disp.downcast_ref()?;
            Some(lt.clone())
        })
    }

    /// Returns the active instance of `Self` if it exists or activates a new instance with the given `config` otherwise.
    /// Activation entails setting the global default [`tracing::Subscriber`], of which there can be only one and it can't
    /// be changed once it is set.
    ///
    /// If a [`LatencyTrace`] has been previously activated in the same process, the `config` passed to this
    /// function will be ignored and the current active [`LatencyTrace`] will be returned.
    ///
    /// # Errors
    /// - [`ActivationError::HistogramConfigError`] if the `config`'s `hist_high` and `hist_sigfig` would cause
    /// [`hdrhistogram::Histogram::new_with_bounds`]`(1, hist_high, hist_sigfig)` to fail.
    /// - [`ActivationError::TracingSubscriberInitError`] if a global [`tracing::Subscriber`] is already set and its
    /// type is not the same as `Self`.
    pub fn activated(config: LatencyTraceCfg) -> Result<Self, ActivationError> {
        config.validate_hist_high_sigfig()?;
        let default_dispatch_exists =
            tracing::dispatcher::get_default(|disp| disp.is::<Layered<Self, Registry>>());
        let lt = if !default_dispatch_exists {
            let lt_new = LatencyTraceG::new(config);
            let reg: Layered<Self, Registry> = Registry::default().with(lt_new.clone());
            reg.try_init()?;
            lt_new
        } else {
            Self::active().expect("`if` condition should ensure `else` Ok")
        };
        Ok(lt)
    }

    /// Returns the active instance of `Self` if it exists or activates a new instance with the default configuration otherwise.
    /// Activation entails setting the global default [`tracing::Subscriber`], of which there can be only one and it can't
    /// be changed once it is set.
    ///
    /// If a [`LatencyTrace`] has been previously activated in the same process, the default configuration
    /// will be ignored and the current active [`LatencyTrace`] will be returned.
    ///
    /// # Errors
    /// - [`ActivationError::TracingSubscriberInitError`] if a global [`tracing::Subscriber`] is already set and its
    /// type is not the same as `Self`.
    pub fn activated_default() -> Result<Self, ActivationError> {
        Self::activated(LatencyTraceCfg::default())
    }
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcJoined,
{
    /// Executes the instrumented function `f` and, after `f` completes, returns the observed latencies.
    pub fn measure_latencies(&self, f: impl FnOnce()) -> Timings {
        f();
        let acc = self.take_acc_timings();
        self.report_timings(acc)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; after `f` completes,
    /// returns the observed latencies.
    pub fn measure_latencies_tokio<F>(self, f: impl FnOnce() -> F) -> Timings
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Tokio runtime error")
                .block_on(f());
        })
    }
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcProbed + Clone,
{
    /// Executes the instrumented function `f`, returning a [`ProbedTrace`] that allows partial latencies to be
    /// reported before `f` completes.
    pub fn measure_latencies_probed(
        self,
        f: impl FnOnce() + Send + 'static,
    ) -> Result<ProbedTraceG<P>, ActivationError> {
        let pt = ProbedTraceG::new(self);
        let jh = thread::spawn(f);
        pt.set_join_handle(jh);
        Ok(pt)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; returns a [`ProbedTrace`]
    /// that allows partial latencies to be reported before `f` completes.
    pub fn measure_latencies_probed_tokio<F>(
        self,
        f: impl FnOnce() -> F + Send + 'static,
    ) -> Result<ProbedTraceG<P>, ActivationError>
    where
        F: Future<Output = ()> + Send,
    {
        self.measure_latencies_probed(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Tokio runtime error")
                .block_on(f());
        })
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
