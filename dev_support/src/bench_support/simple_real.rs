use crate::simple_fns::{simple_real_sync, simple_real_sync_un};
use latency_trace::{
    bench_support::{measure_latencies1, measure_latencies2},
    LatencyTrace, Timings,
};
use std::{fmt::Display, hint::black_box};

pub fn set_up() {
    let lt = LatencyTrace::activated_default().unwrap();
    measure_latencies1(lt);
}

pub fn sync_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::activated_default().unwrap();
    measure_latencies2(lt, move || simple_real_sync(nrepeats, ntasks, sleep_micros));
}

pub fn sync_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTrace::activated_default().unwrap();
    let timings = lt.measure_latencies(move || simple_real_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings)
}

pub fn sync_un_direct(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    simple_real_sync_un(nrepeats, ntasks, sleep_micros);
}

pub fn sync_un_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::activated_default().unwrap();
    measure_latencies2(lt, move || simple_real_sync(nrepeats, ntasks, sleep_micros));
}

pub fn sync_un_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTrace::activated_default().unwrap();
    let timings = lt.measure_latencies(move || simple_real_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings)
}

pub struct Params {
    pub nrepeats: usize,
    pub ntasks: usize,
    pub extent: u64,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Params {
            nrepeats,
            ntasks,
            extent,
        } = self;
        f.write_fmt(format_args!(
            "(nrepeats={nrepeats}, ntasks={ntasks}, extent={extent})"
        ))
    }
}

pub const ARR_PARAMS: [Params; 16] = [
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 10_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 10_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 20_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 20_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 40_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 40_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 80_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 80_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 90_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 90_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 180_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 180_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 110_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 110_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        extent: 120_000,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        extent: 120_000,
    },
];
