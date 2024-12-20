//! [`LatencyTraceG`] activation and measurment methods, other supporting types and/or impls.

use hdrhistogram::CreationError;
use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::Arc,
};
use tracing::Dispatch;
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    util::{SubscriberInitExt, TryInitError},
    Registry,
};

use crate::{
    default_span_grouper,
    lt_collect_g::{LatencyTraceCfg, LatencyTraceG, Timing},
    lt_refine_g::Timings,
    tlc_param::{TlcBase, TlcDirect, TlcParam},
};

//==============
// Errors

/// Error returned by [`LatencyTrace`](crate::LatencyTrace) activation methods.
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
// impl for LatencyTraceCfg

impl LatencyTraceCfg {
    /// Validates that the configuration settings yield histograms that avoid all potential [hdrhistogram::Histogram] errors
    /// as our histograms are `u64`, have a `hist_low` of `1`, and are auto-resizable.
    fn validate_hist_high_sigfig(&self) -> Result<(), CreationError> {
        let _ = Timing::new_with_bounds(1, self.hist_high, self.hist_sigfig)?;
        Ok(())
    }
}

impl Default for LatencyTraceCfg {
    /// Instantiates a default [LatencyTraceCfg]. The defaults are:
    /// - Grouping of spans using the [`default_span_grouper`], which simply groups by the span's
    ///   callsite information, which distills the *tracing* framework's Callsite concept
    ///   (see [Metadata and Callsite](https://docs.rs/tracing-core/0.1.31/tracing_core/)). This default can be
    ///   modified by using the [`Self::with_span_grouper`] method.
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

//==============
// impl for LatencyTrace

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
    ///   [`hdrhistogram::Histogram::new_with_bounds`]`(1, hist_high, hist_sigfig)` to fail.
    /// - [`ActivationError::TracingSubscriberInitError`] if a global [`tracing::Subscriber`] is already set and its
    ///   type is not the same as `Self`.
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
    ///   type is not the same as `Self`.
    pub fn activated_default() -> Result<Self, ActivationError> {
        Self::activated(LatencyTraceCfg::default())
    }
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcDirect,
{
    /// Executes the instrumented function `f` and, after `f` completes, returns the observed latencies.
    pub fn measure_latencies(&self, f: impl FnOnce()) -> Timings {
        f();
        let acc = self.take_acc_timings();
        self.report_timings(acc)
    }
}
