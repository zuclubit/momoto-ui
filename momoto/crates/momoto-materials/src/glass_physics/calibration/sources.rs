//! # Calibration Data Sources
//!
//! Types for different calibration data sources.

// ============================================================================
// SOURCE METADATA
// ============================================================================

/// Metadata about a calibration data source.
#[derive(Debug, Clone)]
pub struct SourceMetadata {
    /// Source name/identifier.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// Original dataset (e.g., "MERL-100").
    pub dataset: Option<String>,
    /// Material name in dataset.
    pub material_name: Option<String>,
    /// Measurement date.
    pub date: Option<String>,
    /// Number of observations.
    pub observation_count: usize,
    /// Data format version.
    pub format_version: Option<String>,
    /// Quality score (0-100).
    pub quality_score: Option<f64>,
}

impl Default for SourceMetadata {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            description: None,
            dataset: None,
            material_name: None,
            date: None,
            observation_count: 0,
            format_version: None,
            quality_score: None,
        }
    }
}

impl SourceMetadata {
    /// Create new metadata with name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Set dataset.
    pub fn with_dataset(mut self, dataset: &str, material: &str) -> Self {
        self.dataset = Some(dataset.to_string());
        self.material_name = Some(material.to_string());
        self
    }

    /// Set observation count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.observation_count = count;
        self
    }

    /// Check if source is from MERL dataset.
    pub fn is_merl(&self) -> bool {
        self.dataset.as_ref().map_or(false, |d| d.contains("MERL"))
    }
}

// ============================================================================
// BRDF OBSERVATIONS
// ============================================================================

/// Single BRDF observation (reflectance at specific geometry).
#[derive(Debug, Clone, Copy)]
pub struct BRDFObservation {
    /// Incident angle theta (radians).
    pub theta_i: f64,
    /// Incident angle phi (radians).
    pub phi_i: f64,
    /// Outgoing angle theta (radians).
    pub theta_o: f64,
    /// Outgoing angle phi (radians).
    pub phi_o: f64,
    /// Measured reflectance (red channel or luminance).
    pub reflectance: f64,
    /// Confidence weight (0-1).
    pub weight: f64,
}

impl BRDFObservation {
    /// Create new observation.
    pub fn new(theta_i: f64, phi_i: f64, theta_o: f64, phi_o: f64, reflectance: f64) -> Self {
        Self {
            theta_i,
            phi_i,
            theta_o,
            phi_o,
            reflectance,
            weight: 1.0,
        }
    }

    /// Create from degrees.
    pub fn from_degrees(
        theta_i: f64,
        phi_i: f64,
        theta_o: f64,
        phi_o: f64,
        reflectance: f64,
    ) -> Self {
        Self::new(
            theta_i.to_radians(),
            phi_i.to_radians(),
            theta_o.to_radians(),
            phi_o.to_radians(),
            reflectance,
        )
    }

    /// Create isotropic observation (phi_i = phi_o = 0).
    pub fn isotropic(theta_i: f64, theta_o: f64, reflectance: f64) -> Self {
        Self::new(theta_i, 0.0, theta_o, 0.0, reflectance)
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Get cosine of incident angle.
    pub fn cos_theta_i(&self) -> f64 {
        self.theta_i.cos()
    }

    /// Get cosine of outgoing angle.
    pub fn cos_theta_o(&self) -> f64 {
        self.theta_o.cos()
    }

    /// Check if this is a specular configuration (theta_i = theta_o, phi_o = phi_i + PI).
    pub fn is_specular(&self, tolerance: f64) -> bool {
        (self.theta_i - self.theta_o).abs() < tolerance
            && (self.phi_o - self.phi_i - std::f64::consts::PI).abs() < tolerance
    }
}

/// BRDF measurement source.
#[derive(Debug, Clone)]
pub struct BRDFSource {
    /// Metadata.
    pub metadata: SourceMetadata,
    /// Observations.
    pub observations: Vec<BRDFObservation>,
    /// Whether data is isotropic.
    pub is_isotropic: bool,
    /// Whether data is specular-only.
    pub is_specular_only: bool,
}

impl BRDFSource {
    /// Create new BRDF source.
    pub fn new(name: &str) -> Self {
        Self {
            metadata: SourceMetadata::new(name),
            observations: Vec::new(),
            is_isotropic: true,
            is_specular_only: false,
        }
    }

    /// Add observation.
    pub fn add_observation(&mut self, obs: BRDFObservation) {
        self.observations.push(obs);
        self.metadata.observation_count = self.observations.len();
    }

    /// Add multiple observations.
    pub fn add_observations(&mut self, obs: &[BRDFObservation]) {
        self.observations.extend_from_slice(obs);
        self.metadata.observation_count = self.observations.len();
    }

    /// Get observations at specific incident angle.
    pub fn at_incident_angle(&self, theta_i: f64, tolerance: f64) -> Vec<&BRDFObservation> {
        self.observations
            .iter()
            .filter(|o| (o.theta_i - theta_i).abs() < tolerance)
            .collect()
    }

    /// Get average reflectance.
    pub fn avg_reflectance(&self) -> f64 {
        if self.observations.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .observations
            .iter()
            .map(|o| o.reflectance * o.weight)
            .sum();
        let weight_sum: f64 = self.observations.iter().map(|o| o.weight).sum();
        sum / weight_sum
    }

    /// Generate uniform angular sampling.
    pub fn from_uniform_sampling(
        name: &str,
        n_theta: usize,
        reflectance_fn: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let mut source = Self::new(name);
        source.is_isotropic = true;

        for i in 0..n_theta {
            let theta_i = (i as f64 + 0.5) * std::f64::consts::FRAC_PI_2 / n_theta as f64;
            for j in 0..n_theta {
                let theta_o = (j as f64 + 0.5) * std::f64::consts::FRAC_PI_2 / n_theta as f64;
                let r = reflectance_fn(theta_i, theta_o);
                source.add_observation(BRDFObservation::isotropic(theta_i, theta_o, r));
            }
        }

        source
    }
}

// ============================================================================
// SPECTRAL OBSERVATIONS
// ============================================================================

/// Single spectral observation.
#[derive(Debug, Clone)]
pub struct SpectralObservation {
    /// Wavelength (nm).
    pub wavelength: f64,
    /// Incident angle (radians).
    pub theta_i: f64,
    /// Measured reflectance.
    pub reflectance: f64,
    /// Measured transmittance (if available).
    pub transmittance: Option<f64>,
    /// Confidence weight.
    pub weight: f64,
}

impl SpectralObservation {
    /// Create new observation.
    pub fn new(wavelength: f64, theta_i: f64, reflectance: f64) -> Self {
        Self {
            wavelength,
            theta_i,
            reflectance,
            transmittance: None,
            weight: 1.0,
        }
    }

    /// Set transmittance.
    pub fn with_transmittance(mut self, t: f64) -> Self {
        self.transmittance = Some(t);
        self
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Get energy sum (R + T).
    pub fn energy_sum(&self) -> f64 {
        self.reflectance + self.transmittance.unwrap_or(0.0)
    }
}

/// Spectral measurement source.
#[derive(Debug, Clone)]
pub struct SpectralSource {
    /// Metadata.
    pub metadata: SourceMetadata,
    /// Observations.
    pub observations: Vec<SpectralObservation>,
    /// Minimum wavelength (nm).
    pub wavelength_min: f64,
    /// Maximum wavelength (nm).
    pub wavelength_max: f64,
    /// Whether transmittance is included.
    pub has_transmittance: bool,
}

impl SpectralSource {
    /// Create new spectral source.
    pub fn new(name: &str) -> Self {
        Self {
            metadata: SourceMetadata::new(name),
            observations: Vec::new(),
            wavelength_min: f64::INFINITY, // Will be set by first observation
            wavelength_max: f64::NEG_INFINITY, // Will be set by first observation
            has_transmittance: false,
        }
    }

    /// Add observation.
    pub fn add_observation(&mut self, obs: SpectralObservation) {
        if obs.transmittance.is_some() {
            self.has_transmittance = true;
        }
        self.wavelength_min = self.wavelength_min.min(obs.wavelength);
        self.wavelength_max = self.wavelength_max.max(obs.wavelength);
        self.observations.push(obs);
        self.metadata.observation_count = self.observations.len();
    }

    /// Get observations at specific wavelength.
    pub fn at_wavelength(&self, wavelength: f64, tolerance: f64) -> Vec<&SpectralObservation> {
        self.observations
            .iter()
            .filter(|o| (o.wavelength - wavelength).abs() < tolerance)
            .collect()
    }

    /// Get spectrum at normal incidence.
    pub fn normal_incidence_spectrum(&self) -> Vec<(f64, f64)> {
        self.observations
            .iter()
            .filter(|o| o.theta_i.abs() < 0.1)
            .map(|o| (o.wavelength, o.reflectance))
            .collect()
    }

    /// Generate from wavelength range.
    pub fn from_wavelength_range(
        name: &str,
        wavelength_min: f64,
        wavelength_max: f64,
        n_samples: usize,
        reflectance_fn: impl Fn(f64) -> f64,
    ) -> Self {
        let mut source = Self::new(name);
        let step = (wavelength_max - wavelength_min) / (n_samples - 1) as f64;

        for i in 0..n_samples {
            let wl = wavelength_min + step * i as f64;
            let r = reflectance_fn(wl);
            source.add_observation(SpectralObservation::new(wl, 0.0, r));
        }

        source
    }
}

// ============================================================================
// TIME SERIES OBSERVATIONS
// ============================================================================

/// Single temporal observation.
#[derive(Debug, Clone)]
pub struct TemporalObservation {
    /// Time (seconds from start).
    pub time: f64,
    /// Frame index.
    pub frame: u64,
    /// Observed reflectance.
    pub reflectance: f64,
    /// Observed transmittance.
    pub transmittance: Option<f64>,
    /// Environmental temperature (if known).
    pub temperature: Option<f64>,
    /// Confidence weight.
    pub weight: f64,
}

impl TemporalObservation {
    /// Create new observation.
    pub fn new(time: f64, frame: u64, reflectance: f64) -> Self {
        Self {
            time,
            frame,
            reflectance,
            transmittance: None,
            temperature: None,
            weight: 1.0,
        }
    }

    /// Set transmittance.
    pub fn with_transmittance(mut self, t: f64) -> Self {
        self.transmittance = Some(t);
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Time series measurement source.
#[derive(Debug, Clone)]
pub struct TimeSeriesSource {
    /// Metadata.
    pub metadata: SourceMetadata,
    /// Observations (sorted by time).
    pub observations: Vec<TemporalObservation>,
    /// Sampling rate (Hz).
    pub sample_rate: f64,
    /// Total duration (seconds).
    pub duration: f64,
    /// Whether transmittance is included.
    pub has_transmittance: bool,
}

impl TimeSeriesSource {
    /// Create new time series source.
    pub fn new(name: &str) -> Self {
        Self {
            metadata: SourceMetadata::new(name),
            observations: Vec::new(),
            sample_rate: 60.0,
            duration: 0.0,
            has_transmittance: false,
        }
    }

    /// Set sample rate.
    pub fn with_sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Add observation.
    pub fn add_observation(&mut self, obs: TemporalObservation) {
        if obs.transmittance.is_some() {
            self.has_transmittance = true;
        }
        self.duration = self.duration.max(obs.time);
        self.observations.push(obs);
        self.metadata.observation_count = self.observations.len();
    }

    /// Sort observations by time.
    pub fn sort(&mut self) {
        self.observations
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Get observation at time (nearest).
    pub fn at_time(&self, time: f64) -> Option<&TemporalObservation> {
        self.observations.iter().min_by(|a, b| {
            let diff_a = (a.time - time).abs();
            let diff_b = (b.time - time).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
    }

    /// Compute drift between first and last observation.
    pub fn total_drift(&self) -> f64 {
        if self.observations.len() < 2 {
            return 0.0;
        }
        let first = self.observations.first().unwrap().reflectance;
        let last = self.observations.last().unwrap().reflectance;
        (last - first).abs()
    }

    /// Generate from time range.
    pub fn from_time_range(
        name: &str,
        duration: f64,
        sample_rate: f64,
        reflectance_fn: impl Fn(f64) -> f64,
    ) -> Self {
        let mut source = Self::new(name).with_sample_rate(sample_rate);
        let dt = 1.0 / sample_rate;
        let n_samples = (duration * sample_rate) as u64;

        for i in 0..n_samples {
            let time = i as f64 * dt;
            let r = reflectance_fn(time);
            source.add_observation(TemporalObservation::new(time, i, r));
        }

        source
    }
}

// ============================================================================
// COMBINED SOURCE
// ============================================================================

/// Combined calibration source from multiple data types.
#[derive(Debug, Clone)]
pub struct CombinedSource {
    /// Metadata.
    pub metadata: SourceMetadata,
    /// BRDF observations (if any).
    pub brdf: Option<BRDFSource>,
    /// Spectral observations (if any).
    pub spectral: Option<SpectralSource>,
    /// Time series observations (if any).
    pub time_series: Option<TimeSeriesSource>,
}

impl CombinedSource {
    /// Create new combined source.
    pub fn new(name: &str) -> Self {
        Self {
            metadata: SourceMetadata::new(name),
            brdf: None,
            spectral: None,
            time_series: None,
        }
    }

    /// Set BRDF source.
    pub fn with_brdf(mut self, source: BRDFSource) -> Self {
        self.brdf = Some(source);
        self.update_count();
        self
    }

    /// Set spectral source.
    pub fn with_spectral(mut self, source: SpectralSource) -> Self {
        self.spectral = Some(source);
        self.update_count();
        self
    }

    /// Set time series source.
    pub fn with_time_series(mut self, source: TimeSeriesSource) -> Self {
        self.time_series = Some(source);
        self.update_count();
        self
    }

    /// Update total observation count.
    fn update_count(&mut self) {
        let mut count = 0;
        if let Some(ref b) = self.brdf {
            count += b.observations.len();
        }
        if let Some(ref s) = self.spectral {
            count += s.observations.len();
        }
        if let Some(ref t) = self.time_series {
            count += t.observations.len();
        }
        self.metadata.observation_count = count;
    }

    /// Check what data types are available.
    pub fn available_types(&self) -> Vec<&'static str> {
        let mut types = Vec::new();
        if self.brdf.is_some() {
            types.push("BRDF");
        }
        if self.spectral.is_some() {
            types.push("Spectral");
        }
        if self.time_series.is_some() {
            types.push("TimeSeries");
        }
        types
    }

    /// Get total observation count.
    pub fn total_observations(&self) -> usize {
        self.metadata.observation_count
    }
}

// ============================================================================
// CALIBRATION SOURCE ENUM
// ============================================================================

/// Unified calibration source enum.
#[derive(Debug, Clone)]
pub enum CalibrationSource {
    /// BRDF measurements.
    BRDF(BRDFSource),
    /// Spectral reflectance.
    Spectral(SpectralSource),
    /// Time series data.
    TimeSeries(TimeSeriesSource),
    /// Combined multi-source data.
    Combined(CombinedSource),
}

impl CalibrationSource {
    /// Get observation count.
    pub fn observation_count(&self) -> usize {
        match self {
            CalibrationSource::BRDF(s) => s.observations.len(),
            CalibrationSource::Spectral(s) => s.observations.len(),
            CalibrationSource::TimeSeries(s) => s.observations.len(),
            CalibrationSource::Combined(s) => s.total_observations(),
        }
    }

    /// Get source name.
    pub fn name(&self) -> &str {
        match self {
            CalibrationSource::BRDF(s) => &s.metadata.name,
            CalibrationSource::Spectral(s) => &s.metadata.name,
            CalibrationSource::TimeSeries(s) => &s.metadata.name,
            CalibrationSource::Combined(s) => &s.metadata.name,
        }
    }

    /// Get source type name.
    pub fn source_type(&self) -> &'static str {
        match self {
            CalibrationSource::BRDF(_) => "BRDF",
            CalibrationSource::Spectral(_) => "Spectral",
            CalibrationSource::TimeSeries(_) => "TimeSeries",
            CalibrationSource::Combined(_) => "Combined",
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_metadata() {
        let meta = SourceMetadata::new("test")
            .with_description("Test source")
            .with_dataset("MERL-100", "gold")
            .with_count(1000);

        assert!(meta.is_merl());
        assert_eq!(meta.observation_count, 1000);
    }

    #[test]
    fn test_brdf_observation() {
        let obs = BRDFObservation::from_degrees(45.0, 0.0, 45.0, 180.0, 0.5);
        assert!(obs.is_specular(0.1));
        assert!((obs.cos_theta_i() - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_brdf_source() {
        let mut source = BRDFSource::new("test");
        source.add_observation(BRDFObservation::isotropic(0.0, 0.0, 0.04));
        source.add_observation(BRDFObservation::isotropic(0.5, 0.5, 0.1));

        assert_eq!(source.observations.len(), 2);
        assert!(source.is_isotropic);
    }

    #[test]
    fn test_brdf_uniform_sampling() {
        let source = BRDFSource::from_uniform_sampling("uniform", 5, |theta_i, _| {
            0.04 + 0.96 * (1.0 - theta_i.cos()).powi(5) // Schlick approx
        });

        assert_eq!(source.observations.len(), 25); // 5x5
    }

    #[test]
    fn test_spectral_observation() {
        let obs = SpectralObservation::new(550.0, 0.0, 0.3).with_transmittance(0.6);

        assert!((obs.energy_sum() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_spectral_source() {
        let source = SpectralSource::from_wavelength_range("test", 400.0, 700.0, 31, |wl| {
            if wl < 500.0 {
                0.1
            } else {
                0.5
            }
        });

        assert_eq!(source.observations.len(), 31);
        assert!((source.wavelength_min - 400.0).abs() < 1.0);
    }

    #[test]
    fn test_temporal_observation() {
        let obs = TemporalObservation::new(1.0, 60, 0.5).with_temperature(25.0);

        assert!(obs.temperature.is_some());
        assert_eq!(obs.frame, 60);
    }

    #[test]
    fn test_time_series_source() {
        let source = TimeSeriesSource::from_time_range("aging", 10.0, 1.0, |t| {
            0.5 * (-t * 0.1).exp() // Exponential decay
        });

        assert_eq!(source.observations.len(), 10);
        assert!(source.total_drift() > 0.0);
    }

    #[test]
    fn test_combined_source() {
        let brdf = BRDFSource::new("brdf");
        let spectral = SpectralSource::new("spectral");

        let combined = CombinedSource::new("combined")
            .with_brdf(brdf)
            .with_spectral(spectral);

        let types = combined.available_types();
        assert!(types.contains(&"BRDF"));
        assert!(types.contains(&"Spectral"));
    }

    #[test]
    fn test_calibration_source_enum() {
        let brdf = BRDFSource::new("test_brdf");
        let source = CalibrationSource::BRDF(brdf);

        assert_eq!(source.source_type(), "BRDF");
        assert_eq!(source.name(), "test_brdf");
    }
}
