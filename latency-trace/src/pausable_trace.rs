use crate::{LatencyTracePriv, Timings};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

/// Represents an ongoing collection of latency information with the ability to be paused before completion.
#[derive(Clone)]
pub struct PausableTrace {
    ltp: LatencyTracePriv,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PausableTrace {
    pub(crate) fn new(ltp: LatencyTracePriv) -> Self {
        Self {
            ltp,
            join_handle: Mutex::new(None).into(),
        }
    }

    pub(crate) fn set_join_handle(&self, join_handle: JoinHandle<()>) {
        let mut lock = self.join_handle.lock();
        let jh = lock.as_deref_mut().unwrap();
        *jh = Some(join_handle);
    }

    pub fn probe_latencies(&self) -> Timings {
        let acc = self.ltp.probe_acc_timings();
        self.ltp.report_timings(acc)
    }

    /// Blocks until the function being measured completes, and then returns the collected latency information.
    ///
    /// Should only be called once, from main thread. May panic otherwise.
    pub fn wait_and_report(&self) -> Timings {
        // try_lock() below should always succeed because this function is the only one that should be joining
        // the handle and it should only be called once from the main thread.
        let join_handle = self.join_handle.try_lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let acc = self.ltp.take_acc_timings();
        self.ltp.report_timings(acc)
    }
}
