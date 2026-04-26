// =============================================================================
// momoto-events::broadcaster — EventBroadcaster pub/sub bus
// =============================================================================

use crate::event::{Event, EventCategory};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Trait for event handlers. Implement this to receive events.
pub trait EventHandler: Send + Sync + 'static {
    /// Called when a matching event is emitted.
    fn handle(&self, event: &Event);

    /// Optional filter. If Some, only matching events are delivered.
    fn filter(&self) -> Option<EventFilter> {
        None
    }
}

/// Filter for event subscriptions.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// If non-empty, only events with these categories are delivered.
    pub categories: Vec<EventCategory>,
    /// If non-empty, only events from these sources are delivered.
    pub sources: Vec<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_categories(mut self, categories: Vec<EventCategory>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_sources(mut self, sources: Vec<String>) -> Self {
        self.sources = sources;
        self
    }

    /// Returns true if the event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        if !self.categories.is_empty() && !self.categories.contains(&event.category) {
            return false;
        }
        if !self.sources.is_empty() && !self.sources.contains(&event.source) {
            return false;
        }
        true
    }
}

/// Configuration for EventBroadcaster.
pub struct BroadcasterConfig {
    pub buffer_size: usize,
    pub enable_buffer: bool,
    pub buffer_max_age_ms: u64,
}

impl Default for BroadcasterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 0,
            enable_buffer: false,
            buffer_max_age_ms: 60_000,
        }
    }
}

/// A subscription handle. Dropping this unsubscribes the handler.
pub struct Subscription {
    id: String,
    broadcaster: Arc<BroadcasterInner>,
}

impl Subscription {
    /// Returns the subscription ID (format: "sub-N").
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut subs = self.broadcaster.subscribers.lock().unwrap();
        subs.remove(&self.id);
    }
}

/// Internal state shared between broadcaster and subscriptions.
struct BroadcasterInner {
    subscribers: Mutex<HashMap<String, Box<dyn EventHandler>>>,
    next_id: Mutex<u64>,
    event_count: Mutex<u64>,
    buffer: Mutex<Vec<Event>>,
    config: BroadcasterConfig,
}

/// Pub/sub event broadcaster. Clone is cheap (Arc-backed).
pub struct EventBroadcaster {
    inner: Arc<BroadcasterInner>,
}

impl EventBroadcaster {
    /// Create with default configuration (no buffer).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(BroadcasterInner {
                subscribers: Mutex::new(HashMap::new()),
                next_id: Mutex::new(0),
                event_count: Mutex::new(0),
                buffer: Mutex::new(Vec::new()),
                config: BroadcasterConfig::default(),
            }),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: BroadcasterConfig) -> Self {
        Self {
            inner: Arc::new(BroadcasterInner {
                subscribers: Mutex::new(HashMap::new()),
                next_id: Mutex::new(0),
                event_count: Mutex::new(0),
                buffer: Mutex::new(Vec::new()),
                config,
            }),
        }
    }

    /// Subscribe a handler. Returns a Subscription that unsubscribes on drop.
    pub fn subscribe<H: EventHandler>(&self, handler: H) -> Subscription {
        let mut id_counter = self.inner.next_id.lock().unwrap();
        *id_counter += 1;
        let id = format!("sub-{}", *id_counter);
        drop(id_counter);

        let mut subs = self.inner.subscribers.lock().unwrap();
        subs.insert(id.clone(), Box::new(handler));
        drop(subs);

        Subscription {
            id,
            broadcaster: Arc::clone(&self.inner),
        }
    }

    /// Emit an event to all matching subscribers.
    pub fn emit(&self, event: Event) {
        // Increment event count
        {
            let mut count = self.inner.event_count.lock().unwrap();
            *count += 1;
        }

        // Buffer if enabled
        if self.inner.config.enable_buffer {
            let mut buf = self.inner.buffer.lock().unwrap();
            if buf.len() < self.inner.config.buffer_size.max(1) {
                buf.push(event.clone());
            }
        }

        // Dispatch to subscribers
        let subs = self.inner.subscribers.lock().unwrap();
        for handler in subs.values() {
            let should_deliver = match handler.filter() {
                Some(filter) => filter.matches(&event),
                None => true,
            };
            if should_deliver {
                handler.handle(&event);
            }
        }
    }

    /// Convenience: emit a progress event.
    pub fn emit_progress(&self, source: &str, progress: f64, message: &str) {
        self.emit(Event::progress(source, progress, message));
    }

    /// Convenience: emit a metric event.
    pub fn emit_metric(&self, source: &str, name: &str, value: f64) {
        self.emit(Event::metric(source, name, value));
    }

    /// Convenience: emit an error event.
    pub fn emit_error(&self, source: &str, description: &str) {
        self.emit(Event::error(source, description));
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.inner.subscribers.lock().unwrap().len()
    }

    /// Returns the total number of events emitted.
    pub fn event_count(&self) -> u64 {
        *self.inner.event_count.lock().unwrap()
    }

    /// Returns all buffered events.
    pub fn buffered_events(&self) -> Vec<Event> {
        self.inner.buffer.lock().unwrap().clone()
    }

    /// Clear the event buffer.
    pub fn clear_buffer(&self) {
        self.inner.buffer.lock().unwrap().clear();
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
