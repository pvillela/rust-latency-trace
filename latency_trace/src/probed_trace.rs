//! Provides the ability to obtain interim timing information before the target function terminates.

use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use crate::{
    core_internals_post::{report_timings, Timings},
    core_internals_pre::LatencyTracePriv,
};

/// Represents an ongoing collection of latency information with the ability to report on partial latencies
/// before the instrumented function completes.
#[derive(Clone)]
pub struct ProbedTrace {
    ltp: LatencyTracePriv,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ProbedTrace {
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

    /// Returns partial latencies collected when the call is made.
    pub fn probe_latencies(&self) -> Timings {
        let acc = self.ltp.probe_acc_timings();
        report_timings(&self.ltp, acc)
    }

    /// Blocks until the function being measured completes, and then returns the collected latency information.
    ///
    /// Should only be called at most once, from main thread. May panic otherwise.
    pub fn wait_and_report(&self) -> Timings {
        // try_lock() below should always succeed because this function is the only one that should be joining
        // the handle and it should only be called once from the main thread.
        let join_handle = self.join_handle.try_lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let acc = self.ltp.take_acc_timings();
        report_timings(&self.ltp, acc)
    }
}
