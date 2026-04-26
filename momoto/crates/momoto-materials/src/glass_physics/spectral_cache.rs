//! Sprint 7 - Spectral Cache for Deterministic Performance
//!
//! Deterministic caching of full spectral evaluation results.
//! Guarantees ΔE=0 for cached configurations with O(1) lookup.
//!
//! ## Key Properties
//! - **Deterministic**: Same inputs always produce same outputs
//! - **Exact**: No interpolation - full spectral accuracy
//! - **Bounded**: LRU eviction keeps memory bounded
//!
//! ## Architecture
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    SpectralCache                                 │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Pipeline Config Hash → (RGB, Energy, Timestamp)                │
//! │  LRU Eviction when capacity reached                             │
//! │  Thread-safe for WASM context                                   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// ============================================================================
// Cache Key
// ============================================================================

/// Quantized cache key for spectral evaluations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpectralCacheKey {
    /// Pipeline configuration hash
    pipeline_hash: u64,
    /// Angle in millidegrees (0-90000)
    angle_mdeg: u32,
    /// Temperature in decikelvin (1000 = 100.0K)
    temp_dk: u32,
}

impl SpectralCacheKey {
    /// Create a cache key from floating-point values
    pub fn new(pipeline_hash: u64, angle_deg: f64, temp_k: f64) -> Self {
        Self {
            pipeline_hash,
            angle_mdeg: (angle_deg.clamp(0.0, 90.0) * 1000.0).round() as u32,
            temp_dk: (temp_k.clamp(0.0, 10000.0) * 10.0).round() as u32,
        }
    }

    /// Create with default temperature (293.15K = room temperature)
    pub fn with_angle(pipeline_hash: u64, angle_deg: f64) -> Self {
        Self::new(pipeline_hash, angle_deg, 293.15)
    }
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Cached spectral evaluation result
#[derive(Debug, Clone)]
pub struct SpectralCacheEntry {
    /// RGB result
    pub rgb: [f64; 3],
    /// Energy ratio (output/input)
    pub energy_ratio: f64,
    /// Access count for LRU
    access_count: u64,
    /// Timestamp for LRU (monotonic counter)
    last_access: u64,
}

impl SpectralCacheEntry {
    pub fn new(rgb: [f64; 3], energy_ratio: f64, timestamp: u64) -> Self {
        Self {
            rgb,
            energy_ratio,
            access_count: 1,
            last_access: timestamp,
        }
    }
}

// ============================================================================
// Spectral Cache
// ============================================================================

/// High-performance spectral result cache
pub struct SpectralCache {
    /// Cache storage
    entries: HashMap<SpectralCacheKey, SpectralCacheEntry>,
    /// Maximum entries before eviction
    max_entries: usize,
    /// Monotonic timestamp for LRU
    timestamp: u64,
    /// Stats
    hits: u64,
    misses: u64,
}

impl SpectralCache {
    /// Create a new cache with specified capacity
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            max_entries,
            timestamp: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Create with default capacity (10000 entries, ~480KB)
    pub fn default_capacity() -> Self {
        Self::new(10000)
    }

    /// Get cached result or None if not found
    pub fn get(&mut self, key: &SpectralCacheKey) -> Option<([f64; 3], f64)> {
        self.timestamp += 1;

        if let Some(entry) = self.entries.get_mut(key) {
            entry.access_count += 1;
            entry.last_access = self.timestamp;
            self.hits += 1;
            Some((entry.rgb, entry.energy_ratio))
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a result into the cache
    pub fn insert(&mut self, key: SpectralCacheKey, rgb: [f64; 3], energy_ratio: f64) {
        self.timestamp += 1;

        // Evict if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_lru();
        }

        let entry = SpectralCacheEntry::new(rgb, energy_ratio, self.timestamp);
        self.entries.insert(key, entry);
    }

    /// Get or compute with caching
    pub fn get_or_compute<F>(&mut self, key: SpectralCacheKey, compute: F) -> ([f64; 3], f64)
    where
        F: FnOnce() -> ([f64; 3], f64),
    {
        if let Some(result) = self.get(&key) {
            return result;
        }

        let result = compute();
        self.insert(key, result.0, result.1);
        result
    }

    /// Evict least recently used entries
    fn evict_lru(&mut self) {
        // Remove 10% of entries with lowest last_access
        let evict_count = (self.max_entries / 10).max(1);

        // Find entries to evict
        let mut entries_vec: Vec<_> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.last_access))
            .collect();
        entries_vec.sort_by_key(|(_, ts)| *ts);

        // Remove oldest entries
        for (key, _) in entries_vec.into_iter().take(evict_count) {
            self.entries.remove(&key);
        }
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Total memory usage in bytes (approximate)
    pub fn memory_bytes(&self) -> usize {
        // Key: 16 bytes, Entry: 40 bytes, HashMap overhead: ~24 bytes per entry
        self.entries.len() * (16 + 40 + 24)
    }

    /// Stats summary
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
            memory_kb: self.memory_bytes() / 1024,
        }
    }
}

impl Default for SpectralCache {
    fn default() -> Self {
        Self::default_capacity()
    }
}

// ============================================================================
// Cache Stats
// ============================================================================

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub memory_kb: usize,
}

impl CacheStats {
    pub fn summary(&self) -> String {
        format!(
            "Cache: {}/{} entries, {:.1}% hit rate, {}KB memory",
            self.entries,
            self.max_entries,
            self.hit_rate * 100.0,
            self.memory_kb
        )
    }
}

// ============================================================================
// Pipeline Hasher
// ============================================================================

/// Helper to generate deterministic hashes for pipeline configurations
pub struct PipelineHasher {
    hasher: std::collections::hash_map::DefaultHasher,
}

impl PipelineHasher {
    pub fn new() -> Self {
        Self {
            hasher: std::collections::hash_map::DefaultHasher::new(),
        }
    }

    /// Add thin film parameters
    pub fn add_thin_film(&mut self, n: f64, thickness_nm: f64, substrate_n: f64) -> &mut Self {
        "thin_film".hash(&mut self.hasher);
        ((n * 1000.0).round() as i64).hash(&mut self.hasher);
        ((thickness_nm * 10.0).round() as i64).hash(&mut self.hasher);
        ((substrate_n * 1000.0).round() as i64).hash(&mut self.hasher);
        self
    }

    /// Add dispersion parameters
    pub fn add_dispersion(&mut self, model: &str, params: &[f64]) -> &mut Self {
        "dispersion".hash(&mut self.hasher);
        model.hash(&mut self.hasher);
        for &p in params {
            ((p * 1e6).round() as i64).hash(&mut self.hasher);
        }
        self
    }

    /// Add metal parameters
    pub fn add_metal(&mut self, metal_type: &str) -> &mut Self {
        "metal".hash(&mut self.hasher);
        metal_type.hash(&mut self.hasher);
        self
    }

    /// Add Mie scattering parameters
    pub fn add_mie(&mut self, g: f64, particle_size_nm: f64) -> &mut Self {
        "mie".hash(&mut self.hasher);
        ((g * 1000.0).round() as i64).hash(&mut self.hasher);
        ((particle_size_nm * 10.0).round() as i64).hash(&mut self.hasher);
        self
    }

    /// Add thermo-optic parameters
    pub fn add_thermo_optic(&mut self, dn_dt: f64) -> &mut Self {
        "thermo_optic".hash(&mut self.hasher);
        ((dn_dt * 1e8).round() as i64).hash(&mut self.hasher);
        self
    }

    /// Get the final hash
    pub fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

impl Default for PipelineHasher {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cached Evaluator
// ============================================================================

use super::spectral_pipeline::*;

/// Evaluator with integrated caching
pub struct CachedSpectralEvaluator {
    cache: SpectralCache,
}

impl CachedSpectralEvaluator {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: SpectralCache::new(max_entries),
        }
    }

    /// Evaluate thin film with caching
    pub fn eval_thin_film(
        &mut self,
        n: f64,
        thickness_nm: f64,
        substrate_n: f64,
        angle_deg: f64,
    ) -> ([f64; 3], f64) {
        let hash = PipelineHasher::new()
            .add_thin_film(n, thickness_nm, substrate_n)
            .finish();
        let key = SpectralCacheKey::with_angle(hash, angle_deg);

        self.cache.get_or_compute(key, || {
            let d65 = SpectralSignal::d65_illuminant();
            let pipeline =
                SpectralPipeline::new().add_stage(ThinFilmStage::new(n, thickness_nm, substrate_n));
            let context = EvaluationContext::default().with_angle_deg(angle_deg);
            let output = pipeline.evaluate(&d65, &context);

            let rgb = output.to_rgb();
            let energy = output.total_energy() / d65.total_energy();
            (rgb, energy)
        })
    }

    /// Evaluate metal with caching
    pub fn eval_metal(&mut self, metal_type: &str, angle_deg: f64) -> ([f64; 3], f64) {
        let hash = PipelineHasher::new().add_metal(metal_type).finish();
        let key = SpectralCacheKey::with_angle(hash, angle_deg);

        self.cache.get_or_compute(key, || {
            let d65 = SpectralSignal::d65_illuminant();
            let stage = match metal_type.to_lowercase().as_str() {
                "gold" => MetalReflectanceStage::gold(),
                "silver" => MetalReflectanceStage::silver(),
                "copper" => MetalReflectanceStage::copper(),
                _ => MetalReflectanceStage::gold(),
            };
            let pipeline = SpectralPipeline::new().add_stage(stage);
            let context = EvaluationContext::default().with_angle_deg(angle_deg);
            let output = pipeline.evaluate(&d65, &context);

            let rgb = output.to_rgb();
            let energy = output.total_energy() / d65.total_energy();
            (rgb, energy)
        })
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for CachedSpectralEvaluator {
    fn default() -> Self {
        Self::new(10000)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cache_basic() {
        let mut cache = SpectralCache::new(100);

        let key = SpectralCacheKey::with_angle(12345, 30.0);
        let rgb = [0.5, 0.6, 0.7];
        let energy = 0.85;

        // Insert
        cache.insert(key.clone(), rgb, energy);

        // Retrieve
        let result = cache.get(&key);
        assert!(result.is_some());
        let (cached_rgb, cached_energy) = result.unwrap();
        assert_eq!(cached_rgb, rgb);
        assert_eq!(cached_energy, energy);
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut cache = SpectralCache::new(100);

        // First access: miss
        let key = SpectralCacheKey::with_angle(12345, 30.0);
        assert!(cache.get(&key).is_none());

        // Insert
        cache.insert(key.clone(), [0.5, 0.5, 0.5], 0.8);

        // Second access: hit
        assert!(cache.get(&key).is_some());

        // Third access: hit
        assert!(cache.get(&key).is_some());

        // 2 hits, 1 miss = 66.7% hit rate
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = SpectralCache::new(10);

        // Fill cache
        for i in 0..10 {
            let key = SpectralCacheKey::with_angle(i as u64, 0.0);
            cache.insert(key, [0.0, 0.0, 0.0], 1.0);
        }
        assert_eq!(cache.len(), 10);

        // Access first few entries to make them "recent"
        for i in 0..5 {
            let key = SpectralCacheKey::with_angle(i as u64, 0.0);
            cache.get(&key);
        }

        // Insert new entry - should trigger eviction
        let new_key = SpectralCacheKey::with_angle(100, 0.0);
        cache.insert(new_key.clone(), [1.0, 1.0, 1.0], 1.0);

        // Cache should still be at or below capacity
        assert!(cache.len() <= 10);

        // New entry should exist
        assert!(cache.get(&new_key).is_some());

        // Recently accessed entries (0-4) should still exist
        for i in 0..5 {
            let key = SpectralCacheKey::with_angle(i as u64, 0.0);
            assert!(
                cache.get(&key).is_some(),
                "Entry {} should still be cached",
                i
            );
        }
    }

    #[test]
    fn test_cached_evaluator_performance() {
        let mut evaluator = CachedSpectralEvaluator::new(1000);

        // Warm up: evaluate multiple configurations
        let configs: Vec<(f64, f64, f64)> = (0..50)
            .map(|i| (1.3 + 0.02 * i as f64, 100.0 + 10.0 * i as f64, 30.0))
            .collect();

        // First pass: all misses (slow)
        let start = Instant::now();
        for (n, t, angle) in &configs {
            evaluator.eval_thin_film(*n, *t, 1.52, *angle);
        }
        let first_pass = start.elapsed();

        // Second pass: all hits (fast)
        let start = Instant::now();
        for (n, t, angle) in &configs {
            evaluator.eval_thin_film(*n, *t, 1.52, *angle);
        }
        let second_pass = start.elapsed();

        let speedup = first_pass.as_nanos() as f64 / second_pass.as_nanos().max(1) as f64;

        println!("\nCached Evaluator Performance:");
        println!("  First pass (cold): {:?}", first_pass);
        println!("  Second pass (hot): {:?}", second_pass);
        println!("  Speedup: {:.1}×", speedup);
        println!("  {}", evaluator.stats().summary());

        // Cache should show ~100% hit rate on second pass
        assert!(evaluator.stats().hit_rate > 0.4, "Expected >40% hit rate");

        // Second pass should be much faster (at least 10×)
        assert!(
            speedup > 10.0,
            "Expected >10× speedup for cached access, got {:.1}×",
            speedup
        );
    }

    #[test]
    fn test_pipeline_hasher() {
        let hash1 = PipelineHasher::new()
            .add_thin_film(1.45, 300.0, 1.52)
            .finish();

        let hash2 = PipelineHasher::new()
            .add_thin_film(1.45, 300.0, 1.52)
            .finish();

        let hash3 = PipelineHasher::new()
            .add_thin_film(1.45, 301.0, 1.52) // Different thickness
            .finish();

        // Same parameters should produce same hash
        assert_eq!(hash1, hash2);

        // Different parameters should produce different hash
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_memory() {
        let cache = SpectralCache::new(10000);
        let empty_memory = cache.memory_bytes();

        println!("\nCache Memory Analysis:");
        println!("  Empty cache: {} bytes", empty_memory);
        println!("  Per entry estimate: {} bytes", (16 + 40 + 24));
        println!("  Max 10000 entries: {} KB", 10000 * (16 + 40 + 24) / 1024);

        // Empty cache should use minimal memory
        assert!(empty_memory < 1024, "Empty cache should use <1KB");
    }

    #[test]
    fn test_delta_e_zero() {
        // Cache guarantees ΔE = 0 (exact results)
        let mut evaluator = CachedSpectralEvaluator::new(100);

        // Evaluate and cache
        let (rgb1, energy1) = evaluator.eval_thin_film(1.45, 300.0, 1.52, 30.0);

        // Retrieve from cache
        let (rgb2, energy2) = evaluator.eval_thin_film(1.45, 300.0, 1.52, 30.0);

        // Should be bit-for-bit identical
        assert_eq!(rgb1, rgb2, "Cached results should be identical");
        assert_eq!(energy1, energy2, "Cached energy should be identical");
    }
}
