use dev_support::deep_fns::{deep_sync, deep_sync_un};
use latency_trace::bench_support::{measure_latencies1, measure_latencies2};
use latency_trace::{LatencyTrace, Timings};
use std::{fmt::Display, hint::black_box, ops::Range};

pub fn set_up() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

pub fn sync_completion(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    let nthreads = measure_latencies2(lt, move || deep_sync(nrepeats, ntasks));
    assert_eq!(nthreads, ntasks + 1, "nthreads == ntasks+1");
}

pub fn sync_all_in(nrepeats: usize, ntasks: usize, exp_span_count: u64) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync(nrepeats, ntasks));
    let span_count = timings.values().fold(0, |acc, hist| acc + hist.len());
    assert_eq!(span_count, exp_span_count, "span_count assertion");
    timings
}

pub fn sync_un_direct(nrepeats: usize, ntasks: usize) {
    deep_sync_un(nrepeats, ntasks);
}

#[allow(unused)]
pub fn sync_un_completion(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || deep_sync_un(nrepeats, ntasks));
}

#[allow(unused)]
pub fn sync_un_all_in(nrepeats: usize, ntasks: usize) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync_un(nrepeats, ntasks));
    black_box(timings)
}

pub struct Params {
    pub nrepeats: usize,
    pub ntasks: usize,
    pub span_count: u64,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Params {
            nrepeats,
            ntasks,
            span_count,
        } = self;
        f.write_fmt(format_args!(
            "(nrepeats={nrepeats}, ntasks={ntasks}, span_count={span_count})"
        ))
    }
}

pub const ARR_PARAMS: [Params; 6] = [
    Params {
        nrepeats: 10,
        ntasks: 0,
        span_count: 94,
    },
    Params {
        nrepeats: 10,
        ntasks: 5,
        span_count: 559,
    },
    Params {
        nrepeats: 20,
        ntasks: 0,
        span_count: 184,
    },
    Params {
        nrepeats: 20,
        ntasks: 5,
        span_count: 1099,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        span_count: 904,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        span_count: 5419,
    },
];

#[allow(unused)]
pub const fn index_range<T, const N: usize>(_arr: &[T; N]) -> Range<usize> {
    Range { start: 0, end: N }
}
