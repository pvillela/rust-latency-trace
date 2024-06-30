//! Types, methods, and functions exported publicly.

use std::{
    error::Error,
    fmt::{Debug, Display},
    future::Future,
    sync::Arc,
    thread,
};

use hdrhistogram::CreationError;
use tracing::span::Attributes;
use tracing_subscriber::{
    layer::{Layered, SubscriberExt},
    util::{SubscriberInitExt, TryInitError},
    Registry,
};

pub use crate::{
    collect::{LatencyTrace, LatencyTraceCfg, Timing},
    refine::{SpanGroup, Timings, TimingsView},
};
use crate::{default_span_grouper, ProbedTrace};

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

impl LatencyTrace {
    /// Returns the active instance of `Self` if it exists.
    pub fn active() -> Option<LatencyTrace> {
        tracing::dispatcher::get_default(|disp| {
            let lt: &LatencyTrace = disp.downcast_ref()?;
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
    pub fn activated(config: LatencyTraceCfg) -> Result<LatencyTrace, ActivationError> {
        config.validate_hist_high_sigfig()?;
        let default_dispatch_exists =
            tracing::dispatcher::get_default(|disp| disp.is::<Layered<LatencyTrace, Registry>>());
        let lt = if !default_dispatch_exists {
            let lt_new = LatencyTrace::new(config);
            let reg: tracing_subscriber::layer::Layered<LatencyTrace, Registry> =
                Registry::default().with(lt_new.clone());
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
    pub fn activated_default() -> Result<LatencyTrace, ActivationError> {
        Self::activated(LatencyTraceCfg::default())
    }

    /// Executes the instrumented function `f` and, after `f` completes, returns the observed latencies.
    pub fn measure_latencies(&self, f: impl FnOnce()) -> Timings {
        f();
        let acc = self.take_acc_timings();
        self.report_timings(acc)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; after `f` completes,
    /// returns the observed latencies.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
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

    /// Executes the instrumented function `f`, returning a [`ProbedTrace`] that allows partial latencies to be
    /// reported before `f` completes.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies_probed(
        self,
        f: impl FnOnce() + Send + 'static,
    ) -> Result<ProbedTrace, ActivationError> {
        let pt = ProbedTrace::new(self);
        let jh = thread::spawn(f);
        pt.set_join_handle(jh);
        Ok(pt)
    }

    /// Executes the instrumented async function `f`, running on the `tokio` runtime; returns a [`ProbedTrace`]
    /// that allows partial latencies to be reported before `f` completes.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with the same `hist_high` and
    /// `hist_sigfic` but a different `span_grouper` then the previous `span_grouper` will be used instead of
    /// the new one.
    ///
    /// # Panics
    ///
    /// If a global default [`tracing::Subscriber`] not provided by this package has been been previously set.
    ///
    /// If a [`LatencyTrace`] has been previously used in the same process with different `hist_high` or
    /// different `hist_sigfic`.
    pub fn measure_latencies_probed_tokio<F>(
        self,
        f: impl FnOnce() -> F + Send + 'static,
    ) -> Result<ProbedTrace, ActivationError>
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
