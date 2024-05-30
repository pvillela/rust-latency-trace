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
            callsite_info_priv_path,
        })
    }
}

/// Intermediate form of latency information collected for span groups, used during post-processing while
/// transforming [`SpanGroupPriv`] to [`SpanGroup`].
type TimingsTemp = HashMap<SpanGroupTemp, Timing>;

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
    for m in acc.into_iter() {
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
pub(crate) fn report_timings(ltp: &LatencyTracePriv, acc: AccTimings) -> Timings {
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
