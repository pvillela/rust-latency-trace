use crate::simple_fns::{simple_async, simple_async_un, simple_sync, simple_sync_un};
use latency_trace::{
    bench_support::{measure_latencies1, measure_latencies2, measure_latencies2_tokio},
    LatencyTraceOld, Timings,
};
use std::{fmt::Display, hint::black_box};

pub fn set_up() {
    let lt = LatencyTraceOld::default();
    measure_latencies1(lt);
}

pub fn sync_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTraceOld::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

pub fn sync_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTraceOld::default();
    let timings = lt
        .measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros))
        .unwrap();
    black_box(timings)
}

pub fn sync_un_direct(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    simple_sync_un(nrepeats, ntasks, sleep_micros);
}

pub fn sync_un_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTraceOld::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

pub fn sync_un_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTraceOld::default();
    let timings = lt
        .measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros))
        .unwrap();
    black_box(timings)
}

pub fn async_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTraceOld::default();
    measure_latencies2_tokio(lt, move || simple_async(nrepeats, ntasks, sleep_micros));
}

pub fn async_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTraceOld::default();
    let timings = lt
        .measure_latencies_tokio(move || simple_async(nrepeats, ntasks, sleep_micros))
        .unwrap();
    black_box(timings);
}

pub fn async_un_direct(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(simple_async_un(nrepeats, ntasks, sleep_micros));
}

pub fn async_un_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTraceOld::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

pub fn async_un_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTraceOld::default();
    let timings = lt
        .measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros))
        .unwrap();
    black_box(timings)
}

pub struct Params {
    pub nrepeats: usize,
    pub ntasks: usize,
    pub sleep_micros: u64,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Params {
            nrepeats,
            ntasks,
            sleep_micros,
        } = self;
        f.write_fmt(format_args!(
            "(nrepeats={nrepeats}, ntasks={ntasks}, sleep_micros={sleep_micros})"
        ))
    }
}

pub const ARR_PARAMS: [Params; 8] = [
    Params {
        nrepeats: 100,
        ntasks: 0,
        sleep_micros: 100,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        sleep_micros: 100,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        sleep_micros: 200,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        sleep_micros: 200,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        sleep_micros: 300,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        sleep_micros: 300,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        sleep_micros: 400,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        sleep_micros: 400,
    },
];
