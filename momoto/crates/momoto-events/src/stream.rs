// =============================================================================
// momoto-events::stream — EventStream for sequential event consumption
// =============================================================================

use crate::broadcaster::{EventBroadcaster, EventHandler, Subscription};
use crate::event::Event;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Stream operating mode.
#[derive(Debug, Clone, Copy, PartialEq)]
enum StreamMode {
    Realtime,
    Batched { batch_size: usize, timeout_ms: u64 },
}

/// Configuration for EventStream.
pub struct StreamConfig {
    mode: StreamMode,
}

impl StreamConfig {
    /// Real-time mode: flush after every event.
    pub fn realtime() -> Self {
        Self {
            mode: StreamMode::Realtime,
        }
    }

    /// Batched mode: accumulate up to `batch_size` events or `timeout_ms`.
    pub fn batched(batch_size: usize, timeout_ms: u64) -> Self {
        Self {
            mode: StreamMode::Batched {
                batch_size,
                timeout_ms,
            },
        }
    }
}

/// Stream lifecycle state.
pub enum StreamState {
    Active,
    Paused,
    Closed,
}

/// Statistics for a stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub total_events: u64,
    pub dropped_events: u64,
    pub pending_count: usize,
    pub state: String,
}

/// A batch of events returned by poll/flush.
pub struct EventBatch {
    pub events: Vec<Event>,
    pub sequence: u64,
    pub total_events: u64,
    pub dropped_events: u64,
}

impl EventBatch {
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Internal state of the stream.
struct StreamInner {
    pending: Vec<Event>,
    total_events: u64,
    dropped_events: u64,
    sequence: u64,
    state: StreamStateInternal,
    config: StreamMode,
}

#[derive(PartialEq)]
enum StreamStateInternal {
    Active,
    Paused,
    Closed,
}

/// EventStream: consumes events from a broadcaster or standalone push.
pub struct EventStream {
    inner: Arc<Mutex<StreamInner>>,
    /// Subscription kept alive while stream is active.
    _subscription: Option<Subscription>,
}

impl EventStream {
    /// Create stream from a broadcaster with config.
    pub fn from_broadcaster(broadcaster: &EventBroadcaster, config: StreamConfig) -> Self {
        let inner = Arc::new(Mutex::new(StreamInner {
            pending: Vec::new(),
            total_events: 0,
            dropped_events: 0,
            sequence: 0,
            state: StreamStateInternal::Active,
            config: config.mode,
        }));

        // Register a handler that pushes events into the stream
        let inner_clone = Arc::clone(&inner);
        struct StreamHandler {
            inner: Arc<Mutex<StreamInner>>,
        }
        impl EventHandler for StreamHandler {
            fn handle(&self, event: &Event) {
                let mut s = self.inner.lock().unwrap();
                if s.state == StreamStateInternal::Active {
                    s.pending.push(event.clone());
                    s.total_events += 1;
                } else {
                    s.dropped_events += 1;
                }
            }
        }

        let sub = broadcaster.subscribe(StreamHandler { inner: inner_clone });

        Self {
            inner,
            _subscription: Some(sub),
        }
    }

    /// Create a standalone stream (no broadcaster, manual push).
    pub fn standalone(config: StreamConfig) -> Self {
        let inner = Arc::new(Mutex::new(StreamInner {
            pending: Vec::new(),
            total_events: 0,
            dropped_events: 0,
            sequence: 0,
            state: StreamStateInternal::Active,
            config: config.mode,
        }));
        Self {
            inner,
            _subscription: None,
        }
    }

    /// Push an event manually (for standalone streams).
    pub fn push(&self, event: Event) -> Result<(), PushError> {
        let mut s = self.inner.lock().unwrap();
        match s.state {
            StreamStateInternal::Active => {
                s.pending.push(event);
                s.total_events += 1;
                Ok(())
            }
            StreamStateInternal::Paused => {
                s.dropped_events += 1;
                Err(PushError::Paused)
            }
            StreamStateInternal::Closed => Err(PushError::Closed),
        }
    }

    /// Poll for available events. Returns Some(EventBatch) if events are ready.
    pub fn poll(&self) -> Option<EventBatch> {
        let mut s = self.inner.lock().unwrap();
        if s.state != StreamStateInternal::Active {
            return None;
        }
        if s.pending.is_empty() {
            return None;
        }
        let should_flush = match s.config {
            StreamMode::Realtime => !s.pending.is_empty(),
            StreamMode::Batched { batch_size, .. } => s.pending.len() >= batch_size,
        };
        if !should_flush {
            return None;
        }
        let events = std::mem::take(&mut s.pending);
        s.sequence += 1;
        Some(EventBatch {
            events,
            sequence: s.sequence,
            total_events: s.total_events,
            dropped_events: s.dropped_events,
        })
    }

    /// Force flush all pending events regardless of batch size.
    pub fn flush(&self) -> Option<EventBatch> {
        let mut s = self.inner.lock().unwrap();
        if s.pending.is_empty() {
            return None;
        }
        let events = std::mem::take(&mut s.pending);
        s.sequence += 1;
        Some(EventBatch {
            events,
            sequence: s.sequence,
            total_events: s.total_events,
            dropped_events: s.dropped_events,
        })
    }

    /// Returns true if the stream should be flushed now.
    pub fn should_flush(&self) -> bool {
        let s = self.inner.lock().unwrap();
        !s.pending.is_empty()
    }

    /// Number of pending (unflushed) events.
    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().pending.len()
    }

    /// Total events received.
    pub fn total_events(&self) -> u64 {
        self.inner.lock().unwrap().total_events
    }

    /// Total events dropped.
    pub fn dropped_events(&self) -> u64 {
        self.inner.lock().unwrap().dropped_events
    }

    /// Current stream state.
    pub fn state(&self) -> StreamState {
        match self.inner.lock().unwrap().state {
            StreamStateInternal::Active => StreamState::Active,
            StreamStateInternal::Paused => StreamState::Paused,
            StreamStateInternal::Closed => StreamState::Closed,
        }
    }

    pub fn pause(&self) {
        let mut s = self.inner.lock().unwrap();
        if s.state == StreamStateInternal::Active {
            s.state = StreamStateInternal::Paused;
        }
    }

    pub fn resume(&self) {
        let mut s = self.inner.lock().unwrap();
        if s.state == StreamStateInternal::Paused {
            s.state = StreamStateInternal::Active;
        }
    }

    pub fn close(&self) {
        let mut s = self.inner.lock().unwrap();
        s.state = StreamStateInternal::Closed;
        s.pending.clear();
    }

    /// Get stream statistics.
    pub fn stats(&self) -> StreamStats {
        let s = self.inner.lock().unwrap();
        StreamStats {
            total_events: s.total_events,
            dropped_events: s.dropped_events,
            pending_count: s.pending.len(),
            state: match s.state {
                StreamStateInternal::Active => "Active".to_string(),
                StreamStateInternal::Paused => "Paused".to_string(),
                StreamStateInternal::Closed => "Closed".to_string(),
            },
        }
    }
}

/// Error type for push operations.
#[derive(Debug)]
pub enum PushError {
    Paused,
    Closed,
}
