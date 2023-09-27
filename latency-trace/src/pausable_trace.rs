use crate::{Latencies, LatencyTracePriv};
use std::{
    sync::{Arc, Mutex, RwLock, TryLockError},
    thread::JoinHandle,
};
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// Defines whether a [`PausableTrace`] operates in [nonblocking](PausableMode::Nonblocking) or
/// [blocking](PausableMode::Blocking) mode.
#[derive(Clone)]
pub enum PausableMode {
    /// Execution of the function being measured continues normally but latency information collection is paused while
    /// the previously collected data is extracted for reporting.
    /// In this case, some latency information is lost during the collection pause. This is the preferred option.
    Nonblocking,
    /// Execution of the function being measured is blocked while the previously collected data is extracted for reporting.
    /// In this case, there is no loss of latency information but there is distortion of latencies for the period during
    /// which execution is paused.
    Blocking,
}

/// Represents an ongoing collection of latency information with the ability to be paused before completion.
#[derive(Clone)]
pub struct PausableTrace {
    ltp: LatencyTracePriv,
    allow_updates: Arc<RwLock<()>>,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    mode: PausableMode,
}

impl PausableTrace {
    pub(crate) fn new(ltp: LatencyTracePriv, mode: PausableMode) -> Self {
        Self {
            ltp,
            allow_updates: RwLock::new(()).into(),
            join_handle: Mutex::new(None).into(),
            mode,
        }
    }

    pub(crate) fn set_join_handle(&self, join_handle: JoinHandle<()>) {
        let mut lock = self.join_handle.lock();
        let jh = lock.as_deref_mut().unwrap();
        *jh = Some(join_handle);
    }

    /// Pauses latency information collection, extracts what has been collected thus far from the various threads,
    /// and returns the results. Latency collection resumes after extraction of the previously collected information.
    pub fn pause_and_report(&self) -> Latencies {
        let allow_updates_lock = self.allow_updates.write().unwrap();
        let mut control_lock = self.ltp.control.lock();
        self.ltp.control.ensure_tls_dropped(&mut control_lock);
        let lp = self.ltp.take_latencies_priv(&mut control_lock);
        drop(control_lock);
        drop(allow_updates_lock);
        self.ltp.generate_latencies(lp)
    }

    /// Blocks until the function being measured completes, and then returns the collected latency information.
    pub fn wait_and_report(&self) -> Latencies {
        let join_handle = self.join_handle.try_lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        let mut control_lock = self.ltp.control.lock();
        self.ltp.control.ensure_tls_dropped(&mut control_lock);
        let lp = self.ltp.take_latencies_priv(&mut control_lock);
        drop(control_lock);
        self.ltp.generate_latencies(lp)
    }
}

impl<S> Layer<S> for PausableTrace
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        self.ltp.on_new_span(attrs, id, ctx)
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        self.ltp.on_enter(id, ctx)
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        self.ltp.on_exit(id, ctx)
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        match self.mode {
            PausableMode::Blocking => {
                let lock_guard = self.allow_updates.read();
                match lock_guard {
                    Ok(_) => self.ltp.on_close(id, ctx),
                    Err(_poison_error) => panic!("poisoned `allow_updates` lock"),
                }
            }

            PausableMode::Nonblocking => {
                let lock_guard = self.allow_updates.try_read();
                match lock_guard {
                    Ok(_) => self.ltp.on_close(id, ctx),
                    Err(TryLockError::WouldBlock) => (),
                    Err(TryLockError::Poisoned(_)) => panic!("poisoned `allow_updates` lock"),
                }
            }
        }
    }
}
