//! Collection of timing information in an efficient way that is not convenient to display.

use hdrhistogram::Histogram;
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use tracing::{callsite::Identifier, span::Attributes, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::tlc_param::{TlcBase, TlcJoined, TlcParam, TlcProbed};

//=================
// Callsite

/// Provides information about where the tracing span was defined.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct CallsiteInfo {
    pub(crate) callsite_id: Identifier,
    pub(crate) name: &'static str,
    pub(crate) file: Option<String>,
    pub(crate) line: Option<u32>,
    pub(crate) parent: Option<Identifier>,
}

//=================
// Paths

// Types used in span groups or to support data collection.

type CallsiteIdPath = Vec<Identifier>;
pub(crate) type Props = Vec<(String, String)>;
type PropsPath = Vec<Arc<Props>>;

/// Private form of [`crate::SpanGroup`] used during trace collection, more efficient than [`crate::SpanGroup`] for trace
/// data collection.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) struct SpanGroupPriv {
    /// Callsite ID of the span group preceded by the callsite IDs of its ancestors.
    pub(crate) callsite_id_path: CallsiteIdPath,

    /// Properties of the span group preceded by the properties of its ancestors.
    pub(crate) props_path: PropsPath,
}

impl SpanGroupPriv {
    pub(crate) fn parent(&self) -> Option<Self> {
        let len = self.callsite_id_path.len();
        if len == 1 {
            return None;
        }
        Some(SpanGroupPriv {
            callsite_id_path: self.callsite_id_path[0..len - 1].into(),
            props_path: self.props_path[0..len - 1].into(),
        })
    }
}

//=================
// Timing and Timings

/// Alias of [`Histogram<u64>`].
pub type Timing = Histogram<u64>;

/// Constructs a [`Timing`]. The arguments correspond to [Histogram::high] and [Histogram::sigfig].
pub(crate) fn new_timing(hist_high: u64, hist_sigfig: u8) -> Timing {
    let mut hist = Histogram::<u64>::new_with_bounds(1, hist_high, hist_sigfig)
        .expect("should not happen given histogram construction");
    hist.auto(true);
    hist
}

#[doc(hidden)]
/// Type of latency information internally collected for span groups. The key is [SpanGroupPriv], which is as
/// light as possible to minimize processing overhead when accessing the map. Therefore, part of the information
/// required to produce the ultimate results is kept as a separate `callsite_infos` map keyed by [`Identifier`].
#[derive(Clone)]
pub struct RawTrace {
    pub(crate) timings: HashMap<SpanGroupPriv, Timing>,
    pub(crate) callsite_infos: HashMap<Identifier, CallsiteInfo>,
}

impl RawTrace {
    pub(crate) fn new() -> Self {
        Self {
            timings: HashMap::new(),
            callsite_infos: HashMap::new(),
        }
    }
}

/// Type of accumulator of thread-local values, prior to transforming the collected information to a [`crate::Timings`].
/// Used to minimize the time holding the control lock during post-processing.
/// The downside is that more memory is used when there are many threads.
// pub(crate) type AccTimings = Vec<HashMap<SpanGroupPriv, TimingPriv>>;
pub(crate) type AccRawTrace = Vec<RawTrace>;

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    // callsite_info_priv_path: CallsiteInfoPrivPath,
    callsite_id_path: CallsiteIdPath,
    props_path: PropsPath,
    created_at: Instant,
}

pub(crate) fn op(raw_trace: RawTrace, acc: &mut AccRawTrace, tid: ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    acc.push(raw_trace);
}

pub(crate) fn op_r(acc1: RawTrace, acc2: RawTrace) -> RawTrace {
    let mut timings = acc1.timings;
    for (k, v) in acc2.timings {
        let hist = timings.get_mut(&k);
        match hist {
            Some(hist) => hist
                .add(v)
                .expect("should not happen given histogram construction"),
            None => {
                timings.insert(k, v);
            }
        }
    }

    let callsite_infos: HashMap<Identifier, CallsiteInfo> = acc1
        .callsite_infos
        .into_iter()
        .chain(acc2.callsite_infos)
        .collect();

    RawTrace {
        timings,
        callsite_infos,
    }
}

//=================
// LatencyTraceCfg

/// Configuration information for [`LatencyTrace`]. It is instantiated with its [`LatencyTraceCfg::default`] method
/// and can be customized with its other methods.
pub struct LatencyTraceCfg {
    pub(crate) span_grouper: SpanGrouper,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

//=================
// SpanGrouper

/// Internal type of span groupers.
type SpanGrouper = Arc<dyn Fn(&Attributes) -> Vec<(String, String)> + Send + Sync + 'static>;

//=================
// LatencyTrace

/// Core type supporting latency mesurements.
///
/// Implements [`tracing_subscriber::Layer`] and provides access to the latencies collected for different span groups.
///
/// There should be a single instance of [`LatencyTrace`] in a process. That instance is set
/// (by method [`Self::activated`] or [`Self::activated_default`])
/// as the global default [`tracing::Subscriber`], of which there can be only one and it can't be changed once
/// it is set.
#[derive(Clone)]
pub struct LatencyTraceG<P>
where
    P: TlcParam,
{
    control: P::Control,
    span_grouper: SpanGrouper,
    pub(crate) hist_high: u64,
    pub(crate) hist_sigfig: u8,
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcBase,
{
    pub(crate) fn new(config: LatencyTraceCfg) -> Self {
        LatencyTraceG {
            control: P::Control::new(),
            span_grouper: config.span_grouper,
            hist_high: config.hist_high,
            hist_sigfig: config.hist_sigfig,
        }
    }

    /// Updates timings for the given span group. Called by [`Layer`] impl.
    fn update_timings(&self, span_group_priv: &SpanGroupPriv, f: impl FnOnce(&mut Timing)) {
        self.control.with_data_mut(|raw_trace| {
            let timing = {
                if let Some(timing) = raw_trace.timings.get_mut(span_group_priv) {
                    timing
                } else {
                    log::trace!(
                        "thread-loacal Timing created for {:?} on {:?}",
                        span_group_priv,
                        thread::current().id()
                    );
                    raw_trace.timings.insert(
                        span_group_priv.clone(),
                        new_timing(self.hist_high, self.hist_sigfig),
                    );
                    raw_trace
                        .timings
                        .get_mut(span_group_priv)
                        .expect("impossible: span_group_priv key was just inserted")
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

    /// Updates callsite info for the given callsite [`Identifier`].
    fn update_callsite_infos(
        &self,
        callsite_id: Identifier,
        callsite_info: impl FnOnce() -> CallsiteInfo,
    ) {
        self.control.with_data_mut(|timings_priv| {
            let callsite_infos = &mut timings_priv.callsite_infos;
            if callsite_infos.get(&callsite_id).is_none() {
                callsite_infos.insert(callsite_id, callsite_info());
            }
        });
    }
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcJoined,
{
    /// Extracts the accumulated timings.
    pub(crate) fn take_acc_timings(&self) -> AccRawTrace {
        log::trace!("entering `take_acc_timings`");
        self.control.take_tls();
        self.control.take_acc(AccRawTrace::new())
    }
}

impl<P> LatencyTraceG<P>
where
    P: TlcParam,
    P::Control: TlcProbed,
{
    pub(crate) fn probe_acc_timings(&self) -> AccRawTrace {
        log::trace!("entering `take_acc_timings`");
        self.control.probe_tls()
    }
}

impl<S, P> Layer<S> for LatencyTraceG<P>
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
    P: TlcParam + 'static,
    P::Control: TlcBase,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx
            .span(id)
            .expect("impossible: there is no span with the given id");
        log::trace!("`on_new_span` start: name={}, id={:?}", span.name(), id);
        let meta = span.metadata();
        let callsite_id = meta.callsite();
        let parent_span = span.parent();

        let props = (self.span_grouper)(attrs);
        let (callsite_id_path, props_path) = match &parent_span {
            None => (vec![callsite_id.clone()], vec![Arc::new(props)]),
            Some(parent_span) => {
                let ext = parent_span.extensions();
                let pst = ext
                    .get::<SpanTiming>()
                    .expect("span extensions does not contain SpanTiming record");
                let mut callsite_id_path = pst.callsite_id_path.clone();
                callsite_id_path.push(callsite_id.clone());
                let mut props_path = pst.props_path.clone();
                props_path.push(Arc::new(props));
                (callsite_id_path, props_path)
            }
        };

        span.extensions_mut().insert(SpanTiming {
            callsite_id_path,
            props_path,
            created_at: Instant::now(),
        });

        let callsite_info = {
            let callsite_id = callsite_id.clone();
            let span = &span;
            move || CallsiteInfo {
                callsite_id,
                name: span.name(),
                file: meta.file().map(|s| s.to_owned()),
                line: meta.line(),
                parent: parent_span
                    .iter()
                    .map(|parent_ref| parent_ref.metadata().callsite())
                    .next(),
            }
        };

        self.update_callsite_infos(callsite_id, callsite_info);

        log::trace!("`on_new_span` end: name={}, id={:?}", span.name(), id);
    }

    // No need for fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {

    // No need for fn on_exit(&self, id: &Id, ctx: Context<'_, S>)

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx
            .span(&id)
            .expect("impossible: there is no span with the given id");
        log::trace!("`on_close` start: name={}, id={:?}", span.name(), id);

        let ext = span.extensions();
        let span_timing = ext
            .get::<SpanTiming>()
            .expect("span extensions does not contain SpanTiming record");

        let span_group_priv = SpanGroupPriv {
            callsite_id_path: span_timing.callsite_id_path.clone(),
            props_path: span_timing.props_path.clone(),
        };

        self.update_timings(&span_group_priv, |hist| {
            hist.record((Instant::now() - span_timing.created_at).as_micros() as u64)
                .expect("should not happen given histogram construction");
        });

        log::trace!(
            "`on_close` completed call to update_timings: name={}, id={:?}",
            span.name(),
            id
        );

        log::trace!("`on_close` end: name={}, id={:?}", span.name(), id);
    }
}
