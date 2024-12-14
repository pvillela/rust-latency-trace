use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
};
use tracing::{span::Attributes, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// Simple [`Layer`] that counts the number of times a span is entered, by span name.
pub struct SimpleSpanCounter(Arc<RwLock<HashMap<String, AtomicU64>>>);

impl SimpleSpanCounter {
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()).into())
    }

    /// Returns the count for a given span name.
    pub fn get(&self, name: &str) -> u64 {
        let lock = self.0.read().expect("unable to get read lock");
        match lock.get(name) {
            Some(v) => v.load(Ordering::Relaxed),
            None => 0,
        }
    }

    fn increment(&self, name: &str) {
        let lock = self.0.read().expect("unable to get read lock");
        match lock.get(name) {
            Some(v) => {
                v.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                drop(lock);
                let mut lock = self.0.write().expect("unable to get write lock");
                lock.insert(name.to_owned(), AtomicU64::new(1));
            }
        }
    }

    /// Returns a map containing the counts for all span names.
    pub fn dump(&self) -> HashMap<String, u64> {
        let lock = self.0.read().expect("unable to get read lock");
        lock.iter()
            .map(|(k, v)| (k.clone(), v.load(Ordering::Relaxed)))
            .collect()
    }
}

impl Clone for SimpleSpanCounter {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> Layer<S> for SimpleSpanCounter
where
    S: Subscriber,
    S: for<'lookup> LookupSpan<'lookup>,
{
    fn on_new_span(&self, _attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx
            .span(id)
            .expect("impossible: there is no span with the given id");
        self.increment(span.name());
    }

    // No need for fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {

    // No need for fn on_exit(&self, id: &Id, ctx: Context<'_, S>)

    // No need for fn on_close(&self, id: Id, ctx: Context<'_, S>) {
}
