use hdrhistogram::Histogram;
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::Arc,
    thread::{self, ThreadId},
    time::Instant,
};
use thread_local_collect::tlm::probed::{Control, Holder};
use tracing::{callsite::Identifier, span::Attributes, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::{LatencyTraceCfg, SpanGrouper, Timing};

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

type CallsiteIdPath = Vec<Identifier>;
type CallsiteInfoPrivPath = Vec<Arc<CallsiteInfoPriv>>;
type Props = Vec<(String, String)>;
type PropsPath = Vec<Arc<Props>>;

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
            callsite_id_path: self.callsite_id_path[0..len - 1].into(),
            props_path: self.props_path[0..len - 1].into(),
        })
    }
}

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

/// Type of latency information internally collected for span groups. The key is [SpanGroupPriv], which is as
/// light as possible to minimize processing overhead when accessing the map. Therefore, part of the information
/// required to produce the ultimate results is kept in the map's values as the `callsite_info_priv_path` field
/// in [TimingsPriv].
type TimingsPriv = HashMap<SpanGroupPriv, TimingPriv>;

/// Type of accumulator of thread-local values, prior to transforming the collected information to a [`Timings`].
/// Used to minimize the time holding the control lock during post-processing.
type AccTimings = Vec<HashMap<SpanGroupPriv, TimingPriv>>;

//=================
// SpanTiming

/// Information about a span stored in the registry.
#[derive(Debug)]
struct SpanTiming {
    callsite_info_priv_path: CallsiteInfoPrivPath,
    props_path: PropsPath,
    created_at: Instant,
}

fn op(timings: TimingsPriv, acc: &mut AccTimings, tid: ThreadId) {
    log::debug!("executing `op` for {:?}", tid);
    acc.push(timings);
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
            control: Control::new(&LOCAL_INFO, AccTimings::new(), TimingsPriv::new, op),
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
                let mut callsite_info_path = pst.callsite_info_priv_path.clone();
                callsite_info_path.push(callsite_info.into());
                let mut props_path = pst.props_path.clone();
                props_path.push(Arc::new(props));
                (callsite_info_path, props_path)
            }
        };

        span.extensions_mut().insert(SpanTiming {
            callsite_info_priv_path: callsite_info_path,
            props_path,
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
                .collect::<Vec<_>>(),
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
