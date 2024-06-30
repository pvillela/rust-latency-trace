//! Provides the ability to obtain interim timing information before the target function terminates.

use crate::{collect::LatencyTrace, refine::Timings};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

/// Represents an ongoing collection of latency information with the ability to report on partial latencies
/// before the instrumented function completes.
#[derive(Clone)]
pub struct ProbedTrace {
    ltp: LatencyTrace,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ProbedTrace {
    pub(crate) fn new(ltp: LatencyTrace) -> Self {
        Self {
            ltp,
            join_handle: Mutex::new(None).into(),
        }
    }

    pub(crate) fn set_join_handle(&self, join_handle: JoinHandle<()>) {
        let mut lock = self.join_handle.lock();
        let jh = lock
            .as_deref_mut()
            .expect("ProbedTrace join_handle Mutex poisoned");
        *jh = Some(join_handle);
    }

    /// Returns partial latencies collected when the call is made.
    pub fn probe_latencies(&self) -> Timings {
        let acc = self.ltp.probe_acc_timings();
        self.ltp.report_timings(acc)
    }

    /// Blocks until the function being measured completes, and then returns the collected latency information.
    ///
    /// Should only be called at most once, from main thread. May panic otherwise.
    pub fn wait_and_report(&self) -> Timings {
        // try_lock() below should always succeed because this function is the only one that should be joining
        // the handle and it should only be called once from the main thread.
        let join_handle = self
            .join_handle
            .try_lock()
            .expect("ProbedTrace lock should not be contended")
            .take()
            .expect("`join_handle` set by constructor, may only be taken once");
        join_handle
            .join()
            .expect("ProbedTrace execution thread exited abnormally");
        let acc = self.ltp.take_acc_timings();
        self.ltp.report_timings(acc)
    }
}
