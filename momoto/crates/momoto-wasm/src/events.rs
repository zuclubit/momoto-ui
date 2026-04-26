// =============================================================================
// momoto-wasm: Events Bindings
// File: crates/momoto-wasm/src/events.rs
//
// Exposes momoto-events crate via wasm-bindgen.
// =============================================================================

use momoto_events::{
    broadcaster::{
        BroadcasterConfig as CoreBroadcasterConfig, EventBroadcaster as CoreBroadcaster,
        EventFilter as CoreFilter, EventHandler,
    },
    event::{Event as CoreEvent, EventCategory as CoreEventCategory},
    stream::{
        EventStream as CoreStream, StreamConfig as CoreStreamConfig, StreamState as CoreStreamState,
    },
};
use wasm_bindgen::prelude::*;

// =============================================================================
// EventBroadcaster — Pub/sub event bus
// =============================================================================

#[wasm_bindgen]
pub struct MomotoEventBus {
    inner: CoreBroadcaster,
}

#[wasm_bindgen]
impl MomotoEventBus {
    /// Create a new event bus with default configuration.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreBroadcaster::new(),
        }
    }

    /// Create with custom buffer size and max age.
    #[wasm_bindgen(js_name = "withConfig")]
    pub fn with_config(buffer_size: usize, buffer_max_age_ms: u64) -> Self {
        Self {
            inner: CoreBroadcaster::with_config(CoreBroadcasterConfig {
                buffer_size,
                enable_buffer: buffer_size > 0,
                buffer_max_age_ms,
            }),
        }
    }

    /// Subscribe with a JS callback. Returns subscriber ID for unsubscribing.
    #[wasm_bindgen]
    pub fn subscribe(&self, callback: js_sys::Function) -> Result<u64, JsValue> {
        struct JsHandler {
            callback: js_sys::Function,
        }

        impl EventHandler for JsHandler {
            fn handle(&self, event: &CoreEvent) {
                if let Ok(json) = event.to_json() {
                    let _ = self
                        .callback
                        .call1(&JsValue::NULL, &JsValue::from_str(&json));
                }
            }
        }

        // Safety: JS functions are Send+Sync in wasm-bindgen single-threaded context
        unsafe impl Send for JsHandler {}
        unsafe impl Sync for JsHandler {}

        let handler = JsHandler { callback };
        let subscription = self.inner.subscribe(handler);
        let id_str = subscription.id().to_string(); // "sub-N"
        let id: u64 = id_str
            .strip_prefix("sub-")
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        std::mem::forget(subscription); // Keep subscription alive
        Ok(id)
    }

    /// Subscribe with category filter.
    #[wasm_bindgen(js_name = "subscribeFiltered")]
    pub fn subscribe_filtered(
        &self,
        categories: &[u8],
        callback: js_sys::Function,
    ) -> Result<u64, JsValue> {
        let filter = CoreFilter::new()
            .with_categories(categories.iter().map(|&c| category_from_u8(c)).collect());

        struct JsFilteredHandler {
            callback: js_sys::Function,
            filter: CoreFilter,
        }

        impl EventHandler for JsFilteredHandler {
            fn handle(&self, event: &CoreEvent) {
                if let Ok(json) = event.to_json() {
                    let _ = self
                        .callback
                        .call1(&JsValue::NULL, &JsValue::from_str(&json));
                }
            }
            fn filter(&self) -> Option<CoreFilter> {
                Some(self.filter.clone())
            }
        }

        unsafe impl Send for JsFilteredHandler {}
        unsafe impl Sync for JsFilteredHandler {}

        let handler = JsFilteredHandler { callback, filter };
        let subscription = self.inner.subscribe(handler);
        let id_str = subscription.id().to_string(); // "sub-N"
        let id: u64 = id_str
            .strip_prefix("sub-")
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        std::mem::forget(subscription);
        Ok(id)
    }

    /// Emit a progress event.
    #[wasm_bindgen(js_name = "emitProgress")]
    pub fn emit_progress(&self, source: &str, progress: f64, message: &str) {
        self.inner.emit_progress(source, progress, message);
    }

    /// Emit a metric event.
    #[wasm_bindgen(js_name = "emitMetric")]
    pub fn emit_metric(&self, source: &str, name: &str, value: f64) {
        self.inner.emit_metric(source, name, value);
    }

    /// Emit an error event.
    #[wasm_bindgen(js_name = "emitError")]
    pub fn emit_error(&self, source: &str, description: &str) {
        self.inner.emit_error(source, description);
    }

    /// Emit a custom event from JSON payload.
    #[wasm_bindgen(js_name = "emitCustom")]
    pub fn emit_custom(&self, source: &str, payload_json: &str) -> Result<(), JsValue> {
        let payload: serde_json::Value = serde_json::from_str(payload_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?;
        let event = CoreEvent::custom(source, payload);
        self.inner.emit(event);
        Ok(())
    }

    /// Emit a full event from JSON.
    #[wasm_bindgen(js_name = "emitJson")]
    pub fn emit_json(&self, event_json: &str) -> Result<(), JsValue> {
        let event: CoreEvent = CoreEvent::from_json(event_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid event JSON: {}", e)))?;
        self.inner.emit(event);
        Ok(())
    }

    /// Get the current subscriber count.
    #[wasm_bindgen(js_name = "subscriberCount")]
    pub fn subscriber_count(&self) -> usize {
        self.inner.subscriber_count()
    }

    /// Get the total event count emitted.
    #[wasm_bindgen(js_name = "eventCount")]
    pub fn event_count(&self) -> u64 {
        self.inner.event_count()
    }

    /// Get all buffered events as JSON array.
    #[wasm_bindgen(js_name = "bufferedEvents")]
    pub fn buffered_events(&self) -> Result<String, JsValue> {
        let events = self.inner.buffered_events();
        let jsons: Vec<String> = events.iter().filter_map(|e| e.to_json().ok()).collect();
        Ok(format!("[{}]", jsons.join(",")))
    }

    /// Clear the event buffer.
    #[wasm_bindgen(js_name = "clearBuffer")]
    pub fn clear_buffer(&self) {
        self.inner.clear_buffer();
    }
}

// =============================================================================
// MomotoEventStream — Streaming event consumption
// =============================================================================

#[wasm_bindgen]
pub struct MomotoEventStream {
    inner: CoreStream,
}

#[wasm_bindgen]
impl MomotoEventStream {
    /// Create a real-time stream from an event bus.
    #[wasm_bindgen(js_name = "fromBus")]
    pub fn from_bus(bus: &MomotoEventBus) -> Self {
        Self {
            inner: CoreStream::from_broadcaster(&bus.inner, CoreStreamConfig::realtime()),
        }
    }

    /// Create a batched stream (more efficient for high-throughput).
    #[wasm_bindgen(js_name = "fromBusBatched")]
    pub fn from_bus_batched(bus: &MomotoEventBus, batch_size: usize, timeout_ms: u64) -> Self {
        Self {
            inner: CoreStream::from_broadcaster(
                &bus.inner,
                CoreStreamConfig::batched(batch_size, timeout_ms),
            ),
        }
    }

    /// Create a standalone stream (no bus, manual push).
    #[wasm_bindgen]
    pub fn standalone() -> Self {
        Self {
            inner: CoreStream::standalone(CoreStreamConfig::realtime()),
        }
    }

    /// Push an event into the stream (for standalone streams).
    #[wasm_bindgen]
    pub fn push(&self, event_json: &str) -> Result<(), JsValue> {
        let event: CoreEvent = CoreEvent::from_json(event_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid event: {}", e)))?;
        self.inner
            .push(event)
            .map_err(|e| JsValue::from_str(&format!("Push error: {:?}", e)))
    }

    /// Poll for available events. Returns JSON or null.
    #[wasm_bindgen]
    pub fn poll(&self) -> Result<JsValue, JsValue> {
        match self.inner.poll() {
            Some(batch) => {
                let events: Vec<String> = batch
                    .events
                    .iter()
                    .filter_map(|e| e.to_json().ok())
                    .collect();
                let json = serde_json::json!({
                    "events": serde_json::Value::Array(
                        events.iter()
                            .filter_map(|s| serde_json::from_str(s).ok())
                            .collect()
                    ),
                    "sequence": batch.sequence,
                    "totalEvents": batch.total_events,
                    "droppedEvents": batch.dropped_events,
                    "count": batch.len(),
                });
                Ok(JsValue::from_str(&json.to_string()))
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Force flush pending events.
    #[wasm_bindgen]
    pub fn flush(&self) -> Result<JsValue, JsValue> {
        match self.inner.flush() {
            Some(batch) => {
                let events: Vec<String> = batch
                    .events
                    .iter()
                    .filter_map(|e| e.to_json().ok())
                    .collect();
                Ok(JsValue::from_str(&format!("[{}]", events.join(","))))
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Check if flush should happen.
    #[wasm_bindgen(js_name = "shouldFlush")]
    pub fn should_flush(&self) -> bool {
        self.inner.should_flush()
    }

    /// Number of pending events.
    #[wasm_bindgen(js_name = "pendingCount")]
    pub fn pending_count(&self) -> usize {
        self.inner.pending_count()
    }

    /// Total events processed.
    #[wasm_bindgen(js_name = "totalEvents")]
    pub fn total_events(&self) -> u64 {
        self.inner.total_events()
    }

    /// Total events dropped.
    #[wasm_bindgen(js_name = "droppedEvents")]
    pub fn dropped_events(&self) -> u64 {
        self.inner.dropped_events()
    }

    /// Current stream state.
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> String {
        match self.inner.state() {
            CoreStreamState::Active => "Active".to_string(),
            CoreStreamState::Paused => "Paused".to_string(),
            CoreStreamState::Closed => "Closed".to_string(),
        }
    }

    #[wasm_bindgen]
    pub fn pause(&self) {
        self.inner.pause();
    }

    #[wasm_bindgen]
    pub fn resume(&self) {
        self.inner.resume();
    }

    #[wasm_bindgen]
    pub fn close(&self) {
        self.inner.close();
    }

    /// Get stream stats as JSON.
    #[wasm_bindgen]
    pub fn stats(&self) -> Result<String, JsValue> {
        let stats = self.inner.stats();
        serde_json::to_string(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn category_from_u8(v: u8) -> CoreEventCategory {
    match v {
        0 => CoreEventCategory::Progress,
        1 => CoreEventCategory::Metrics,
        2 => CoreEventCategory::Recommendation,
        3 => CoreEventCategory::Validation,
        4 => CoreEventCategory::Error,
        5 => CoreEventCategory::System,
        6 => CoreEventCategory::Chart,
        7 => CoreEventCategory::Heartbeat,
        _ => CoreEventCategory::Custom,
    }
}
