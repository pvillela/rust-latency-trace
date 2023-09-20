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

#[derive(Clone)]
pub struct PausableTrace {
    ltp: LatencyTracePriv,
    allow_updates: Arc<RwLock<()>>,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl PausableTrace {
    pub(crate) fn new(ltp: LatencyTracePriv) -> Self {
        Self {
            ltp,
            allow_updates: RwLock::new(()).into(),
            join_handle: Mutex::new(None).into(),
        }
    }

    pub(crate) fn set_join_handle(&self, join_handle: JoinHandle<()>) {
        let mut lock = self.join_handle.lock();
        let jh = lock.as_deref_mut().unwrap();
        *jh = Some(join_handle);
    }

    pub fn pause_and_collect(&self) -> Latencies {
        let _lock_guard = self.allow_updates.write().unwrap();
        self.ltp.control.ensure_tls_dropped();
        self.ltp.generate_latencies()
    }

    pub fn wait_and_collect(&self) -> Latencies {
        let join_handle = self.join_handle.try_lock().unwrap().take().unwrap();
        join_handle.join().unwrap();
        self.ltp.control.ensure_tls_dropped();
        self.ltp.generate_latencies()
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
        let lock_guard = self.allow_updates.try_read();
        match lock_guard {
            Ok(_) => self.ltp.on_close(id, ctx),
            Err(TryLockError::WouldBlock) => (),
            Err(TryLockError::Poisoned(_)) => panic!("poisoned `allow_updates` lock"),
        }
    }
}
