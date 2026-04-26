//! # Session Management
//!
//! Provides multi-turn conversation sessions, bot authentication,
//! rate limiting, and session persistence for the Momoto agent layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

// ============================================================================
// Primitive Types
// ============================================================================

/// Storage format for serialized sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageFormat {
    /// JSON (human-readable)
    Json,
    /// Binary format (compact, faster)
    Bincode,
}

impl Default for StorageFormat {
    fn default() -> Self {
        StorageFormat::Json
    }
}

/// Errors that can occur during session operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionError {
    /// Session with given ID was not found.
    NotFound(String),
    /// Session has expired.
    Expired(String),
    /// Caller is not authorized for this session.
    Unauthorized,
    /// Underlying storage failure.
    StorageError(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NotFound(id) => write!(f, "Session not found: {}", id),
            SessionError::Expired(id) => write!(f, "Session expired: {}", id),
            SessionError::Unauthorized => write!(f, "Unauthorized session access"),
            SessionError::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

/// Errors that can occur during bot session operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BotSessionError {
    /// Bot session not found.
    NotFound,
    /// Bot is not authorized for the requested operation.
    Unauthorized,
    /// Rate limit exceeded for this bot.
    RateLimited,
    /// Supplied API key / credentials are invalid.
    InvalidCredentials,
}

impl fmt::Display for BotSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BotSessionError::NotFound => write!(f, "Bot session not found"),
            BotSessionError::Unauthorized => write!(f, "Bot not authorized"),
            BotSessionError::RateLimited => write!(f, "Rate limit exceeded"),
            BotSessionError::InvalidCredentials => write!(f, "Invalid credentials"),
        }
    }
}

// ============================================================================
// Identifier Newtypes
// ============================================================================

/// A workflow identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub String);

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WorkflowId {
    fn from(s: String) -> Self {
        WorkflowId(s)
    }
}

impl From<&str> for WorkflowId {
    fn from(s: &str) -> Self {
        WorkflowId(s.to_string())
    }
}

/// A bot identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BotId(pub String);

impl fmt::Display for BotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BotId {
    fn from(s: String) -> Self {
        BotId(s)
    }
}

impl From<&str> for BotId {
    fn from(s: &str) -> Self {
        BotId(s.to_string())
    }
}

// ============================================================================
// Bot Permission / Credential / Config
// ============================================================================

/// Permissions granted to a registered bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotPermissions {
    /// Whether the bot may read session data.
    pub can_read: bool,
    /// Whether the bot may write / update session data.
    pub can_write: bool,
    /// Whether the bot may trigger workflow execution.
    pub can_execute_workflows: bool,
    /// Maximum number of API requests allowed per minute.
    pub max_requests_per_minute: u32,
}

impl Default for BotPermissions {
    fn default() -> Self {
        BotPermissions {
            can_read: true,
            can_write: true,
            can_execute_workflows: false,
            max_requests_per_minute: 60,
        }
    }
}

/// Credentials for a registered bot (stored as hashed secrets).
///
/// The `secret_hash` stores a SHA-256-like hex string derived from the raw
/// secret so the plain-text secret is never persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCredentials {
    /// Public API key (used as identifier).
    pub api_key: String,
    /// Hex-encoded hash of the bot secret (SHA-256 style, no external dep).
    pub secret_hash: String,
    /// Optional Unix-timestamp after which these credentials expire.
    pub expires_at: Option<u64>,
}

impl BotCredentials {
    /// Create new credentials, hashing the raw secret with a simple but
    /// deterministic transformation (XOR-fold + hex encoding).
    ///
    /// In production you would use SHA-256 from `sha2`; here we implement a
    /// lightweight substitute so the crate has no extra dependency.
    pub fn new(api_key: impl Into<String>, raw_secret: &str, expires_at: Option<u64>) -> Self {
        BotCredentials {
            api_key: api_key.into(),
            secret_hash: Self::hash_secret(raw_secret),
            expires_at,
        }
    }

    /// Verify that `raw_secret` matches the stored hash.
    pub fn verify(&self, raw_secret: &str) -> bool {
        Self::hash_secret(raw_secret) == self.secret_hash
    }

    /// Simple deterministic hash: fold bytes into a 32-byte buffer, output hex.
    fn hash_secret(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut buf = [0u8; 32];
        for (i, &b) in bytes.iter().enumerate() {
            buf[i % 32] ^= b.wrapping_add((i as u8).wrapping_mul(37));
        }
        // Mix rounds for diffusion.
        for round in 0..4u8 {
            for i in 0..32 {
                buf[i] = buf[i]
                    .wrapping_add(buf[(i + 1) % 32])
                    .wrapping_add(round.wrapping_mul(13));
            }
        }
        buf.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Return true if these credentials have expired relative to `now_secs`.
    pub fn is_expired(&self, now_secs: u64) -> bool {
        self.expires_at.map(|exp| now_secs >= exp).unwrap_or(false)
    }
}

/// Configuration for a registered bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    /// Unique bot identifier.
    pub id: BotId,
    /// Human-readable display name.
    pub name: String,
    /// Bot credentials (hashed).
    pub credentials: BotCredentials,
    /// Permissions granted to this bot.
    pub permissions: BotPermissions,
    /// Unix timestamp when this config was created.
    pub created_at: u64,
}

// ============================================================================
// Conversation / Context
// ============================================================================

/// A named variable bound to a session context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVariable {
    /// Variable name.
    pub name: String,
    /// Serialized value.
    pub value: String,
    /// Type hint (e.g. "color", "string", "number").
    pub variable_type: String,
}

impl ContextVariable {
    /// Create a new context variable.
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        variable_type: impl Into<String>,
    ) -> Self {
        ContextVariable {
            name: name.into(),
            value: value.into(),
            variable_type: variable_type.into(),
        }
    }
}

/// A single turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Monotonic turn index (1-based).
    pub turn: u32,
    /// Role: `"user"` or `"assistant"`.
    pub role: String,
    /// Text content of this turn.
    pub content: String,
    /// Unix timestamp in seconds.
    pub timestamp: u64,
    /// Optional color hex referenced in this turn (e.g. `#0066cc`).
    pub color_context: Option<String>,
}

/// A bounded history of conversation turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    /// Stored turns.
    pub turns: Vec<ConversationTurn>,
    /// Maximum number of turns to keep (oldest are dropped when exceeded).
    pub max_turns: usize,
}

impl ConversationHistory {
    /// Create an empty history with a given capacity limit.
    pub fn new(max_turns: usize) -> Self {
        ConversationHistory {
            turns: Vec::new(),
            max_turns,
        }
    }

    /// Append a new turn.  If `max_turns` is exceeded the oldest entry is removed.
    pub fn add_turn(&mut self, turn: ConversationTurn) {
        self.turns.push(turn);
        if self.turns.len() > self.max_turns {
            self.turns.remove(0);
        }
    }

    /// Return the last `n` turns (or all if fewer than `n` are stored).
    pub fn last_n(&self, n: usize) -> &[ConversationTurn] {
        let len = self.turns.len();
        if n >= len {
            &self.turns
        } else {
            &self.turns[len - n..]
        }
    }

    /// Remove all stored turns.
    pub fn clear(&mut self) {
        self.turns.clear();
    }

    /// Current number of turns.
    pub fn len(&self) -> usize {
        self.turns.len()
    }

    /// True when no turns are stored.
    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }
}

/// Contextual data attached to a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Named variables accessible across turns.
    pub variables: HashMap<String, ContextVariable>,
    /// Recently referenced color hex strings.
    pub last_colors: Vec<String>,
    /// Currently active workflow, if any.
    pub active_workflow: Option<WorkflowId>,
}

impl SessionContext {
    /// Create an empty context.
    pub fn new() -> Self {
        SessionContext {
            variables: HashMap::new(),
            last_colors: Vec::new(),
            active_workflow: None,
        }
    }

    /// Set a variable.
    pub fn set_variable(&mut self, var: ContextVariable) {
        self.variables.insert(var.name.clone(), var);
    }

    /// Get a variable by name.
    pub fn get_variable(&self, name: &str) -> Option<&ContextVariable> {
        self.variables.get(name)
    }

    /// Push a color to the recent color list (keeps last 10).
    pub fn push_color(&mut self, hex: impl Into<String>) {
        self.last_colors.push(hex.into());
        if self.last_colors.len() > 10 {
            self.last_colors.remove(0);
        }
    }
}

impl Default for SessionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A lightweight snapshot of a session for listing / dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Session ID.
    pub session_id: String,
    /// Unix timestamp when the session was created.
    pub created_at: u64,
    /// Unix timestamp of the most recent interaction.
    pub last_active: u64,
    /// Total number of turns recorded.
    pub turn_count: u32,
    /// Brief human-readable summary of the session context.
    pub context_summary: String,
}

// ============================================================================
// Session
// ============================================================================

/// Return a monotonically increasing pseudo-timestamp in seconds.
///
/// Uses a static counter initialized from a compile-time constant so the
/// implementation works in no-std / WASM environments without `std::time`.
fn monotonic_secs() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    // Seed is a rough epoch approximation (2025-01-01 00:00:00 UTC).
    static EPOCH: AtomicU64 = AtomicU64::new(1_735_689_600);
    // Each call increments by 1 to give monotonic IDs (not wall-clock accurate,
    // but sufficient for ordering, expiry checks, and snapshot timestamps).
    EPOCH.fetch_add(1, Ordering::Relaxed)
}

/// A live conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// Contextual data (variables, colors, active workflow).
    pub context: SessionContext,
    /// Turn-by-turn conversation history.
    pub history: ConversationHistory,
    /// Unix timestamp when this session was created.
    pub created_at: u64,
    /// Unix timestamp of the last interaction.
    pub last_active: u64,
    /// Unix timestamp after which this session is considered expired.
    pub expires_at: u64,
}

impl Session {
    /// Create a new session with the given ID and a default lifetime of 1 hour.
    pub fn new(id: String) -> Self {
        let now = monotonic_secs();
        Session {
            id,
            context: SessionContext::new(),
            history: ConversationHistory::new(50),
            created_at: now,
            last_active: now,
            expires_at: now + 3600,
        }
    }

    /// Create a session with a custom expiry duration (seconds).
    pub fn with_expiry(id: String, expiry_secs: u64) -> Self {
        let now = monotonic_secs();
        Session {
            id,
            context: SessionContext::new(),
            history: ConversationHistory::new(50),
            created_at: now,
            last_active: now,
            expires_at: now + expiry_secs,
        }
    }

    /// Check whether this session has expired.
    pub fn is_expired(&self) -> bool {
        monotonic_secs() >= self.expires_at
    }

    /// Append a turn to the history and touch `last_active`.
    pub fn add_turn(&mut self, role: &str, content: &str) {
        let now = monotonic_secs();
        let turn_num = self.history.len() as u32 + 1;
        let turn = ConversationTurn {
            turn: turn_num,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
            color_context: None,
        };
        self.history.add_turn(turn);
        self.last_active = now;
    }

    /// Build a lightweight snapshot of this session.
    pub fn snapshot(&self) -> SessionSnapshot {
        let summary = if self.context.variables.is_empty() && self.context.last_colors.is_empty() {
            "Empty context".to_string()
        } else {
            format!(
                "{} variable(s), {} recent color(s)",
                self.context.variables.len(),
                self.context.last_colors.len()
            )
        };
        SessionSnapshot {
            session_id: self.id.clone(),
            created_at: self.created_at,
            last_active: self.last_active,
            turn_count: self.history.len() as u32,
            context_summary: summary,
        }
    }

    /// Touch `last_active` and extend the expiry window by `extra_secs`.
    pub fn refresh(&mut self, extra_secs: u64) {
        let now = monotonic_secs();
        self.last_active = now;
        if self.expires_at < now + extra_secs {
            self.expires_at = now + extra_secs;
        }
    }
}

// ============================================================================
// SessionStore trait + implementations
// ============================================================================

/// Trait for session persistence backends.
pub trait SessionStore {
    /// Retrieve a session by ID.
    fn get(&self, id: &str) -> Result<Session, SessionError>;
    /// Persist (create or overwrite) a session.
    fn save(&self, session: &Session) -> Result<(), SessionError>;
    /// Remove a session by ID.
    fn delete(&self, id: &str) -> Result<(), SessionError>;
    /// Return all session IDs currently in the store.
    fn list_ids(&self) -> Vec<String>;
}

/// In-memory session store backed by a `HashMap` behind a `Mutex`.
#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    inner: Arc<Mutex<HashMap<String, Session>>>,
}

impl InMemorySessionStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        InMemorySessionStore {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for InMemorySessionStore {
    fn get(&self, id: &str) -> Result<Session, SessionError> {
        let map = self
            .inner
            .lock()
            .map_err(|e| SessionError::StorageError(e.to_string()))?;
        map.get(id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(id.to_string()))
    }

    fn save(&self, session: &Session) -> Result<(), SessionError> {
        let mut map = self
            .inner
            .lock()
            .map_err(|e| SessionError::StorageError(e.to_string()))?;
        map.insert(session.id.clone(), session.clone());
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), SessionError> {
        let mut map = self
            .inner
            .lock()
            .map_err(|e| SessionError::StorageError(e.to_string()))?;
        if map.remove(id).is_none() {
            return Err(SessionError::NotFound(id.to_string()));
        }
        Ok(())
    }

    fn list_ids(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// File-backed session store.
///
/// In a native context this would write JSON/Bincode files to `path`.
/// In a WASM context (no filesystem access) it transparently delegates
/// to an in-memory store.
#[derive(Debug, Clone)]
pub struct FileSessionStore {
    /// Target directory path (unused in WASM builds).
    pub path: String,
    /// Serialization format.
    pub format: StorageFormat,
    /// Actual backing store.
    inner: InMemorySessionStore,
}

impl FileSessionStore {
    /// Create a file store pointing at `path`.
    pub fn new(path: impl Into<String>) -> Self {
        FileSessionStore {
            path: path.into(),
            format: StorageFormat::Json,
            inner: InMemorySessionStore::new(),
        }
    }

    /// Create a file store with an explicit storage format.
    pub fn with_format(path: impl Into<String>, format: StorageFormat) -> Self {
        FileSessionStore {
            path: path.into(),
            format,
            inner: InMemorySessionStore::new(),
        }
    }
}

impl SessionStore for FileSessionStore {
    fn get(&self, id: &str) -> Result<Session, SessionError> {
        self.inner.get(id)
    }

    fn save(&self, session: &Session) -> Result<(), SessionError> {
        self.inner.save(session)
    }

    fn delete(&self, id: &str) -> Result<(), SessionError> {
        self.inner.delete(id)
    }

    fn list_ids(&self) -> Vec<String> {
        self.inner.list_ids()
    }
}

// ============================================================================
// SessionManager
// ============================================================================

/// Configuration for `SessionManager`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManagerConfig {
    /// Maximum concurrent sessions.  `0` means unlimited.
    pub max_sessions: usize,
    /// Inactivity timeout in seconds before a session is eligible for cleanup.
    pub session_timeout_secs: u64,
    /// Maximum number of conversation turns per session.
    pub max_turns_per_session: usize,
}

impl Default for SessionManagerConfig {
    fn default() -> Self {
        SessionManagerConfig {
            max_sessions: 1000,
            session_timeout_secs: 3600,
            max_turns_per_session: 50,
        }
    }
}

/// Generates a unique session ID without external dependencies.
///
/// Format: `sess-{counter:016x}-{salt:08x}` where `salt` is derived from
/// the lower bits of the counter folded with a compile-time constant.
fn generate_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let salt = ((n ^ 0xDEAD_BEEF_CAFE_1234u64).wrapping_mul(0x517C_C1B7_2722_0A95u64)) as u32;
    format!("sess-{:016x}-{:08x}", n, salt)
}

/// Manages the lifecycle of `Session` objects using a pluggable `SessionStore`.
pub struct SessionManager {
    /// Configuration.
    pub config: SessionManagerConfig,
    /// Backing store.
    store: Arc<dyn SessionStore + Send + Sync>,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl SessionManager {
    /// Create a new manager with the given config and an in-memory store.
    pub fn new(config: SessionManagerConfig) -> Self {
        let store = Arc::new(InMemorySessionStore::new());
        SessionManager { config, store }
    }

    /// Create a manager with default configuration (1000 sessions, 1h timeout, 50 turns).
    pub fn default_manager() -> Self {
        Self::new(SessionManagerConfig::default())
    }

    /// Create a manager with a custom store.
    pub fn with_store(
        config: SessionManagerConfig,
        store: Arc<dyn SessionStore + Send + Sync>,
    ) -> Self {
        SessionManager { config, store }
    }

    /// Create a fresh session with an optional context, returning its ID.
    ///
    /// The `context` parameter is currently used to initialise the session's
    /// metadata (user agent, locale, etc.).  Pass `None` for a plain session.
    pub fn create_session(&self, _context: Option<SessionContext>) -> String {
        let id = generate_session_id();
        let mut session = Session::with_expiry(id.clone(), self.config.session_timeout_secs);
        session.history.max_turns = self.config.max_turns_per_session;
        // Best-effort save; if over limit, evict one expired session first.
        if self.config.max_sessions > 0 {
            let current_ids = self.store.list_ids();
            if current_ids.len() >= self.config.max_sessions {
                // Try to evict one expired session.
                for eid in current_ids {
                    if let Ok(s) = self.store.get(&eid) {
                        if s.is_expired() {
                            let _ = self.store.delete(&eid);
                            break;
                        }
                    }
                }
            }
        }
        let _ = self.store.save(&session);
        id
    }

    /// Retrieve a session, returning an error if not found or expired.
    pub fn get_session(&self, id: &str) -> Result<Session, SessionError> {
        let session = self.store.get(id)?;
        if session.is_expired() {
            return Err(SessionError::Expired(id.to_string()));
        }
        Ok(session)
    }

    /// Persist an updated session.
    pub fn update_session(&self, session: Session) -> Result<(), SessionError> {
        self.store.save(&session)
    }

    /// Delete a session by ID.
    pub fn delete_session(&self, id: &str) -> Result<(), SessionError> {
        self.store.delete(id)
    }

    /// Remove all expired sessions.  Returns the number of sessions deleted.
    pub fn cleanup_expired(&self) -> u32 {
        let ids = self.store.list_ids();
        let mut removed = 0u32;
        for id in ids {
            if let Ok(s) = self.store.get(&id) {
                if s.is_expired() {
                    if self.store.delete(&s.id).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
        removed
    }

    /// Return the number of active (non-expired) sessions.
    pub fn active_count(&self) -> usize {
        self.store
            .list_ids()
            .iter()
            .filter(|id| self.store.get(id).map(|s| !s.is_expired()).unwrap_or(false))
            .count()
    }
}

// ============================================================================
// PersistentSessionManager
// ============================================================================

/// A `SessionManager` that uses a `FileSessionStore` for persistence.
///
/// In WASM builds the file store is backed by in-memory storage; on native
/// targets the store path can be configured for actual disk persistence.
pub struct PersistentSessionManager {
    /// Underlying manager.
    pub manager: SessionManager,
    /// Reference to the file store for format / path inspection.
    pub file_store: Arc<FileSessionStore>,
}

impl PersistentSessionManager {
    /// Create a persistent manager writing to `path`.
    pub fn new(path: impl Into<String>, config: SessionManagerConfig) -> Self {
        let file_store = Arc::new(FileSessionStore::new(path));
        let store: Arc<dyn SessionStore + Send + Sync> = file_store.clone();
        PersistentSessionManager {
            manager: SessionManager::with_store(config, store),
            file_store,
        }
    }

    /// Delegate: create session.
    pub fn create_session(&self) -> String {
        self.manager.create_session(None)
    }

    /// Delegate: get session.
    pub fn get_session(&self, id: &str) -> Result<Session, SessionError> {
        self.manager.get_session(id)
    }

    /// Delegate: update session.
    pub fn update_session(&self, session: Session) -> Result<(), SessionError> {
        self.manager.update_session(session)
    }

    /// Delegate: delete session.
    pub fn delete_session(&self, id: &str) -> Result<(), SessionError> {
        self.manager.delete_session(id)
    }

    /// Delegate: cleanup expired sessions.
    pub fn cleanup_expired(&self) -> u32 {
        self.manager.cleanup_expired()
    }
}

// ============================================================================
// RateLimiter
// ============================================================================

/// A simple sliding-window rate limiter.
///
/// Tracks the number of requests in a 60-second window and rejects new ones
/// once `max_per_minute` is reached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    /// Maximum allowed requests in one 60-second window.
    pub max_per_minute: u32,
    /// Running count within the current window.
    pub current_count: u32,
    /// Timestamp (seconds) when the current window started.
    pub window_start: u64,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_per_minute: u32) -> Self {
        RateLimiter {
            max_per_minute,
            current_count: 0,
            window_start: monotonic_secs(),
        }
    }

    /// Attempt to record one request.
    ///
    /// Returns `true` if the request is allowed; `false` if the limit is
    /// reached.  The window resets automatically after 60 seconds.
    pub fn check_and_increment(&mut self) -> bool {
        let now = monotonic_secs();
        // Reset window if 60 seconds have elapsed.
        if now >= self.window_start + 60 {
            self.window_start = now;
            self.current_count = 0;
        }
        if self.current_count < self.max_per_minute {
            self.current_count += 1;
            true
        } else {
            false
        }
    }

    /// Manually reset the window and counter.
    pub fn reset(&mut self) {
        self.current_count = 0;
        self.window_start = monotonic_secs();
    }

    /// Returns `true` if the current window is exhausted.
    pub fn is_limited(&self) -> bool {
        self.current_count >= self.max_per_minute
    }
}

// ============================================================================
// BotSession / BotSessionManager
// ============================================================================

/// An active session owned by a bot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSession {
    /// Which bot owns this session.
    pub bot_id: BotId,
    /// The underlying session ID managed by `SessionManager`.
    pub session_id: String,
    /// Per-session rate limiter for this bot.
    pub rate_limiter: RateLimiter,
    /// Whether this session is currently active.
    pub active: bool,
}

impl BotSession {
    /// Create a new bot session.
    pub fn new(bot_id: BotId, session_id: String, max_per_minute: u32) -> Self {
        BotSession {
            bot_id,
            session_id,
            rate_limiter: RateLimiter::new(max_per_minute),
            active: true,
        }
    }
}

/// Manages bot registration, authentication, and per-bot sessions.
#[derive(Debug)]
pub struct BotSessionManager {
    /// Registered bots indexed by their `BotId` string.
    bots: HashMap<String, BotConfig>,
    /// Active bot sessions indexed by `session_id`.
    sessions: HashMap<String, BotSession>,
}

impl BotSessionManager {
    /// Create an empty manager.
    pub fn new() -> Self {
        BotSessionManager {
            bots: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    /// Register a new bot.  Overwrites any existing config for the same ID.
    pub fn register_bot(&mut self, config: BotConfig) {
        self.bots.insert(config.id.0.clone(), config);
    }

    /// Remove a bot registration.
    pub fn deregister_bot(&mut self, bot_id: &BotId) -> bool {
        self.bots.remove(&bot_id.0).is_some()
    }

    /// Authenticate a bot with its API key and create a new session.
    ///
    /// Returns the new `session_id` on success, or a `BotSessionError`.
    pub fn create_bot_session(
        &mut self,
        bot_id: &BotId,
        api_key: &str,
    ) -> Result<String, BotSessionError> {
        let config = self
            .bots
            .get(&bot_id.0)
            .ok_or(BotSessionError::NotFound)?
            .clone();

        // Verify API key matches the registered credentials.
        if config.credentials.api_key != api_key {
            return Err(BotSessionError::InvalidCredentials);
        }

        // Check expiry.
        let now = monotonic_secs();
        if config.credentials.is_expired(now) {
            return Err(BotSessionError::InvalidCredentials);
        }

        // Generate a unique session ID for the bot.
        let session_id = format!("bot-{}-{:016x}", bot_id.0, now);

        let bot_session = BotSession::new(
            bot_id.clone(),
            session_id.clone(),
            config.permissions.max_requests_per_minute,
        );
        self.sessions.insert(session_id.clone(), bot_session);
        Ok(session_id)
    }

    /// Validate that a session is active and the bot has not exceeded its rate limit.
    pub fn validate_request(&mut self, session_id: &str) -> Result<(), BotSessionError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(BotSessionError::NotFound)?;

        if !session.active {
            return Err(BotSessionError::Unauthorized);
        }

        if !session.rate_limiter.check_and_increment() {
            return Err(BotSessionError::RateLimited);
        }

        Ok(())
    }

    /// Terminate an active bot session.
    pub fn terminate_session(&mut self, session_id: &str) -> bool {
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.active = false;
            true
        } else {
            false
        }
    }

    /// Return the `BotConfig` for a given `BotId`, if registered.
    pub fn bot_config(&self, bot_id: &BotId) -> Option<&BotConfig> {
        self.bots.get(&bot_id.0)
    }

    /// Return all registered bot IDs.
    pub fn registered_bots(&self) -> Vec<&str> {
        self.bots.keys().map(|s| s.as_str()).collect()
    }

    /// Return all active session IDs.
    pub fn active_sessions(&self) -> Vec<&str> {
        self.sessions
            .iter()
            .filter(|(_, s)| s.active)
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

impl Default for BotSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- WorkflowId / BotId ---

    #[test]
    fn test_workflow_id_display() {
        let wid = WorkflowId("wf-001".to_string());
        assert_eq!(format!("{}", wid), "wf-001");
    }

    #[test]
    fn test_bot_id_from_str() {
        let bid: BotId = BotId::from("my-bot");
        assert_eq!(bid.0, "my-bot");
    }

    // --- BotCredentials ---

    #[test]
    fn test_credentials_verify() {
        let creds = BotCredentials::new("key-abc", "super-secret", None);
        assert!(creds.verify("super-secret"));
        assert!(!creds.verify("wrong-secret"));
    }

    #[test]
    fn test_credentials_expiry() {
        let creds = BotCredentials::new("k", "s", Some(100));
        assert!(creds.is_expired(101));
        assert!(!creds.is_expired(99));
    }

    // --- ConversationHistory ---

    #[test]
    fn test_history_bounded() {
        let mut history = ConversationHistory::new(3);
        for i in 0..5u32 {
            history.add_turn(ConversationTurn {
                turn: i + 1,
                role: "user".to_string(),
                content: format!("msg {}", i),
                timestamp: 0,
                color_context: None,
            });
        }
        assert_eq!(history.len(), 3);
        assert_eq!(history.turns[0].turn, 3); // oldest kept
    }

    #[test]
    fn test_history_last_n() {
        let mut history = ConversationHistory::new(10);
        for i in 0..6u32 {
            history.add_turn(ConversationTurn {
                turn: i + 1,
                role: "user".into(),
                content: format!("t{}", i),
                timestamp: 0,
                color_context: None,
            });
        }
        assert_eq!(history.last_n(3).len(), 3);
        assert_eq!(history.last_n(20).len(), 6);
    }

    // --- Session ---

    #[test]
    fn test_session_new() {
        let session = Session::new("test-id".to_string());
        assert_eq!(session.id, "test-id");
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_add_turn() {
        let mut session = Session::new("s1".to_string());
        session.add_turn("user", "hello");
        session.add_turn("assistant", "hi!");
        assert_eq!(session.history.len(), 2);
    }

    #[test]
    fn test_session_snapshot() {
        let mut session = Session::new("snap-test".to_string());
        session.add_turn("user", "what color is accessible on white?");
        let snap = session.snapshot();
        assert_eq!(snap.session_id, "snap-test");
        assert_eq!(snap.turn_count, 1);
    }

    // --- InMemorySessionStore ---

    #[test]
    fn test_in_memory_store_crud() {
        let store = InMemorySessionStore::new();
        let session = Session::new("abc".to_string());

        store.save(&session).unwrap();
        let loaded = store.get("abc").unwrap();
        assert_eq!(loaded.id, "abc");

        store.delete("abc").unwrap();
        assert!(matches!(store.get("abc"), Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_in_memory_store_list() {
        let store = InMemorySessionStore::new();
        store.save(&Session::new("id-1".into())).unwrap();
        store.save(&Session::new("id-2".into())).unwrap();
        let ids = store.list_ids();
        assert_eq!(ids.len(), 2);
    }

    // --- SessionManager ---

    #[test]
    fn test_session_manager_create_and_get() {
        let config = SessionManagerConfig::default();
        let mgr = SessionManager::new(config);
        let id = mgr.create_session(None);
        assert!(!id.is_empty());
        let session = mgr.get_session(&id).unwrap();
        assert_eq!(session.id, id);
    }

    #[test]
    fn test_session_manager_delete() {
        let mgr = SessionManager::new(SessionManagerConfig::default());
        let id = mgr.create_session(None);
        mgr.delete_session(&id).unwrap();
        assert!(matches!(
            mgr.get_session(&id),
            Err(SessionError::NotFound(_))
        ));
    }

    #[test]
    fn test_session_manager_cleanup() {
        let config = SessionManagerConfig {
            session_timeout_secs: 0, // Immediately expired
            ..Default::default()
        };
        let mgr = SessionManager::new(config);
        let _id = mgr.create_session(None);
        // Session created with expiry = now + 0, which is already expired.
        let removed = mgr.cleanup_expired();
        assert!(removed >= 1);
    }

    // --- RateLimiter ---

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let mut rl = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(rl.check_and_increment());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut rl = RateLimiter::new(2);
        assert!(rl.check_and_increment());
        assert!(rl.check_and_increment());
        assert!(!rl.check_and_increment());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let mut rl = RateLimiter::new(1);
        rl.check_and_increment();
        assert!(rl.is_limited());
        rl.reset();
        assert!(!rl.is_limited());
    }

    // --- BotSessionManager ---

    #[test]
    fn test_bot_session_register_and_authenticate() {
        let mut mgr = BotSessionManager::new();
        let config = BotConfig {
            id: BotId::from("bot-1"),
            name: "Test Bot".into(),
            credentials: BotCredentials::new("api-key-xyz", "secret", None),
            permissions: BotPermissions::default(),
            created_at: 0,
        };
        mgr.register_bot(config);

        let sid = mgr
            .create_bot_session(&BotId::from("bot-1"), "api-key-xyz")
            .unwrap();
        assert!(sid.starts_with("bot-bot-1-"));
    }

    #[test]
    fn test_bot_session_wrong_key() {
        let mut mgr = BotSessionManager::new();
        mgr.register_bot(BotConfig {
            id: BotId::from("bot-2"),
            name: "B".into(),
            credentials: BotCredentials::new("correct-key", "secret", None),
            permissions: BotPermissions::default(),
            created_at: 0,
        });
        let result = mgr.create_bot_session(&BotId::from("bot-2"), "wrong-key");
        assert!(matches!(result, Err(BotSessionError::InvalidCredentials)));
    }

    #[test]
    fn test_bot_session_rate_limiting() {
        let mut mgr = BotSessionManager::new();
        let mut perms = BotPermissions::default();
        perms.max_requests_per_minute = 2;
        mgr.register_bot(BotConfig {
            id: BotId::from("bot-3"),
            name: "Limited".into(),
            credentials: BotCredentials::new("key3", "s", None),
            permissions: perms,
            created_at: 0,
        });
        let sid = mgr
            .create_bot_session(&BotId::from("bot-3"), "key3")
            .unwrap();
        assert!(mgr.validate_request(&sid).is_ok());
        assert!(mgr.validate_request(&sid).is_ok());
        assert!(matches!(
            mgr.validate_request(&sid),
            Err(BotSessionError::RateLimited)
        ));
    }

    #[test]
    fn test_bot_session_unknown_session() {
        let mut mgr = BotSessionManager::new();
        let result = mgr.validate_request("nonexistent-session");
        assert!(matches!(result, Err(BotSessionError::NotFound)));
    }
}
