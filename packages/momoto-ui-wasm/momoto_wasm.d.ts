/* tslint:disable */
/* eslint-disable */

export class APCAMetric {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new APCA metric.
   */
  constructor();
  /**
   * Evaluate APCA contrast (Lc value) between foreground and background.
   *
   * Returns Lc value:
   * - Positive = dark text on light background
   * - Negative = light text on dark background
   * - Near zero = insufficient contrast
   */
  evaluate(foreground: Color, background: Color): ContrastResult;
  /**
   * Evaluate APCA contrast for multiple color pairs (faster than calling evaluate in a loop).
   *
   * # Arguments
   *
   * * `foregrounds` - Array of foreground colors
   * * `backgrounds` - Array of background colors (must match length)
   *
   * # Returns
   *
   * Array of APCA results with Lc values and polarities
   */
  evaluateBatch(foregrounds: Color[], backgrounds: Color[]): ContrastResult[];
}

export enum AccessibilityModeEnum {
  HighContrast = 0,
  ReducedMotion = 1,
  ReducedTransparency = 2,
  InvertedColors = 3,
}

export class AdvancedScore {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  recommendationStrength(): number;
  isStrongRecommendation(): boolean;
  /**
   * Returns "Critical", "High", "Medium", or "Low".
   */
  priorityAssessment(): string;
  readonly qualityOverall: number;
  readonly impact: number;
  readonly effort: number;
  readonly confidence: number;
  readonly priority: number;
  /**
   * Full breakdown as JSON.
   */
  readonly breakdown: any;
}

export class AdvancedScorer {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Score a recommendation with impact, effort, confidence analysis.
   */
  scoreRecommendation(category: string, before_overall: number, before_compliance: number, before_perceptual: number, before_appropriateness: number, after_overall: number, after_compliance: number, after_perceptual: number, after_appropriateness: number, delta_l: number, delta_c: number, delta_h: number): AdvancedScore;
}

export class AgentExecutor {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Execute a query and return the response as JSON.
   */
  execute(query_json: string): string;
}

export class AmbientShadowParams {
  free(): void;
  [Symbol.dispose](): void;
  constructor(base_opacity: number, blur_radius: number, offset_y: number, spread: number);
  static standard(): AmbientShadowParams;
  static elevated(): AmbientShadowParams;
  static subtle(): AmbientShadowParams;
  static dramatic(): AmbientShadowParams;
}

export class BackgroundContext {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * White background preset
   */
  static white(): BackgroundContext;
  /**
   * Black background preset
   */
  static black(): BackgroundContext;
  /**
   * Gray background preset
   */
  static gray(): BackgroundContext;
  /**
   * Colorful background preset
   */
  static colorful(): BackgroundContext;
  /**
   * Sky background preset
   */
  static sky(): BackgroundContext;
}

export class BatchEvaluator {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new batch evaluator with default context
   */
  constructor();
  /**
   * Create batch evaluator with custom context
   */
  static withContext(context: MaterialContext): BatchEvaluator;
  /**
   * Evaluate batch of materials
   *
   * Returns result object with arrays for each property.
   * This is 7-10x faster than evaluating materials individually
   * when called from JavaScript (reduces JS↔WASM crossings).
   */
  evaluate(input: BatchMaterialInput): BatchResult;
  /**
   * Update context
   */
  setContext(context: MaterialContext): void;
}

export class BatchMaterialInput {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new empty batch input
   */
  constructor();
  /**
   * Add a material to the batch
   *
   * # Arguments
   *
   * * `ior` - Index of refraction
   * * `roughness` - Surface roughness (0-1)
   * * `thickness` - Thickness in mm
   * * `absorption` - Absorption coefficient per mm
   */
  push(ior: number, roughness: number, thickness: number, absorption: number): void;
  /**
   * Get number of materials in batch
   */
  len(): number;
  /**
   * Check if batch is empty
   */
  isEmpty(): boolean;
}

export class BatchResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get opacity array
   */
  getOpacity(): Float64Array;
  /**
   * Get blur array
   */
  getBlur(): Float64Array;
  /**
   * Get Fresnel normal incidence array
   */
  getFresnelNormal(): Float64Array;
  /**
   * Get Fresnel grazing angle array
   */
  getFresnelGrazing(): Float64Array;
  /**
   * Get transmittance array
   */
  getTransmittance(): Float64Array;
  /**
   * Number of materials evaluated
   */
  readonly count: number;
}

/**
 * Blur intensity levels matching Apple HIG.
 */
export enum BlurIntensity {
  /**
   * No blur (0px)
   */
  None = 0,
  /**
   * Light blur (10px)
   */
  Light = 1,
  /**
   * Medium blur (20px)
   */
  Medium = 2,
  /**
   * Heavy blur (30px)
   */
  Heavy = 3,
  /**
   * Extra heavy blur (40px)
   */
  ExtraHeavy = 4,
}

export class CauchyDispersion {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create custom Cauchy dispersion model.
   *
   * # Arguments
   *
   * * `a` - Base refractive index (A coefficient, typically ~1.5)
   * * `b` - First dispersion coefficient (B coefficient, nm²)
   * * `c` - Second dispersion coefficient (C coefficient, nm⁴)
   */
  constructor(a: number, b: number, c: number);
  /**
   * Create from base IOR with default dispersion.
   *
   * Uses empirical relationship between IOR and dispersion.
   */
  static fromIor(ior: number): CauchyDispersion;
  /**
   * Create non-dispersive model (constant IOR).
   */
  static constant(ior: number): CauchyDispersion;
  /**
   * Crown glass (BK7) - Low dispersion optical glass.
   * Abbe number ~64
   */
  static crownGlass(): CauchyDispersion;
  /**
   * Flint glass (SF11) - High dispersion dense glass.
   * Abbe number ~25
   */
  static flintGlass(): CauchyDispersion;
  /**
   * Fused silica - Very low dispersion, pure SiO2.
   * Abbe number ~68
   */
  static fusedSilica(): CauchyDispersion;
  /**
   * Water at 20°C.
   * Abbe number ~56
   */
  static water(): CauchyDispersion;
  /**
   * Diamond - Very high dispersion ("fire").
   * Abbe number ~44
   */
  static diamond(): CauchyDispersion;
  /**
   * Polycarbonate (PC) - High dispersion plastic.
   * Abbe number ~30
   */
  static polycarbonate(): CauchyDispersion;
  /**
   * PMMA (Acrylic) - Low dispersion plastic.
   * Abbe number ~57
   */
  static pmma(): CauchyDispersion;
  /**
   * Calculate refractive index at given wavelength.
   *
   * # Arguments
   *
   * * `wavelength_nm` - Wavelength in nanometers (visible: 380-780nm)
   *
   * # Returns
   *
   * Refractive index n (typically 1.0 to 2.5)
   */
  n(wavelength_nm: number): number;
  /**
   * Calculate refractive indices for RGB channels.
   *
   * Uses standard wavelengths: R=656.3nm, G=587.6nm, B=486.1nm
   *
   * # Returns
   *
   * Array [n_red, n_green, n_blue]
   */
  nRgb(): Float64Array;
  /**
   * Calculate Abbe number (dispersion strength).
   *
   * V_d = (n_d - 1) / (n_F - n_C)
   *
   * Higher values = less dispersion (crown glass ~60)
   * Lower values = more dispersion (flint glass ~30)
   */
  abbeNumber(): number;
  /**
   * Get base refractive index (at sodium d-line, 589.3nm).
   */
  nBase(): number;
  /**
   * Base coefficient A (approximate IOR at d-line).
   */
  readonly a: number;
  /**
   * First dispersion coefficient B (nm²).
   */
  readonly b: number;
  /**
   * Second dispersion coefficient C (nm⁴).
   */
  readonly c: number;
}

export class Color {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a color from RGB values (0-255).
   */
  constructor(r: number, g: number, b: number);
  /**
   * Create a color from hex string (e.g., "#FF0000" or "FF0000").
   */
  static fromHex(hex: string): Color;
  /**
   * Convert to hex string (e.g., "#FF0000").
   */
  toHex(): string;
  /**
   * Create a new Color with the specified alpha (opacity) value.
   *
   * # Arguments
   * * `alpha` - Alpha value (0.0 = transparent, 1.0 = opaque)
   *
   * # Example (JavaScript)
   * ```javascript
   * const color = Color.fromHex("#FF0000");
   * const semiTransparent = color.withAlpha(0.5);
   * console.log(semiTransparent.alpha); // 0.5
   * ```
   */
  withAlpha(alpha: number): Color;
  /**
   * Make the color lighter by the specified amount.
   *
   * # Arguments
   * * `amount` - Lightness increase (0.0 to 1.0)
   */
  lighten(amount: number): Color;
  /**
   * Make the color darker by the specified amount.
   *
   * # Arguments
   * * `amount` - Lightness decrease (0.0 to 1.0)
   */
  darken(amount: number): Color;
  /**
   * Increase the saturation (chroma) of the color.
   *
   * # Arguments
   * * `amount` - Chroma increase
   */
  saturate(amount: number): Color;
  /**
   * Decrease the saturation (chroma) of the color.
   *
   * # Arguments
   * * `amount` - Chroma decrease
   */
  desaturate(amount: number): Color;
  /**
   * Get red channel (0-255).
   */
  readonly r: number;
  /**
   * Get green channel (0-255).
   */
  readonly g: number;
  /**
   * Get blue channel (0-255).
   */
  readonly b: number;
  /**
   * Get the alpha (opacity) value of this color (0.0-1.0).
   *
   * Returns 1.0 for fully opaque colors.
   */
  readonly alpha: number;
}

export enum ColorSpaceEnum {
  SRgb = 0,
  DisplayP3 = 1,
  Rec2020 = 2,
  LinearRgb = 3,
}

export class ComplexIOR {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new complex IOR.
   *
   * # Arguments
   *
   * * `n` - Real part (refractive index)
   * * `k` - Imaginary part (extinction coefficient)
   */
  constructor(n: number, k: number);
  /**
   * Create dielectric (k = 0).
   */
  static dielectric(n: number): ComplexIOR;
  /**
   * Calculate F0 (normal incidence reflectance).
   *
   * F0 = ((n-1)² + k²) / ((n+1)² + k²)
   */
  f0(): number;
  /**
   * Check if this is a conductor (has significant extinction).
   */
  isConductor(): boolean;
  /**
   * Calculate penetration depth (skin depth) in nanometers.
   */
  penetrationDepthNm(wavelength_nm: number): number;
  /**
   * Real part: refractive index.
   */
  readonly n: number;
  /**
   * Imaginary part: extinction coefficient.
   */
  readonly k: number;
}

/**
 * Target compliance level for recommendations.
 */
export enum ComplianceTarget {
  /**
   * WCAG 2.1 Level AA (minimum legal requirement in many jurisdictions)
   */
  WCAG_AA = 0,
  /**
   * WCAG 2.1 Level AAA (enhanced accessibility)
   */
  WCAG_AAA = 1,
  /**
   * APCA-based recommendations (modern perceptual contrast)
   */
  APCA = 2,
  /**
   * Meet both WCAG AA and APCA minimums
   */
  Hybrid = 3,
}

export class ConductorBSDF {
  free(): void;
  [Symbol.dispose](): void;
  constructor(n: number, k: number, roughness: number);
  static gold(): ConductorBSDF;
  static silver(): ConductorBSDF;
  static copper(): ConductorBSDF;
  static aluminum(): ConductorBSDF;
  static chrome(): ConductorBSDF;
  evaluate(wi_x: number, wi_y: number, wi_z: number, wo_x: number, wo_y: number, wo_z: number): any;
  validateEnergy(): any;
}

export class ConstraintValidator {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  static withConfig(energy_tolerance: number, reciprocity_tolerance: number, max_spectral_gradient: number, hard_clamp: boolean): ConstraintValidator;
}

export class ContactShadow {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Convert to CSS box-shadow string.
   *
   * # Example output
   *
   * `"0 0.5px 2.0px 0.0px oklch(0.050 0.003 240.0 / 0.75)"`
   */
  toCss(): string;
  /**
   * Get the shadow color as OKLCH.
   */
  readonly color: OKLCH;
  /**
   * Get blur radius in pixels.
   */
  readonly blur: number;
  /**
   * Get vertical offset in pixels.
   */
  readonly offsetY: number;
  /**
   * Get spread in pixels.
   */
  readonly spread: number;
  /**
   * Get opacity (0.0-1.0).
   */
  readonly opacity: number;
}

export class ContactShadowParams {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create contact shadow params with custom values.
   *
   * # Arguments
   *
   * * `darkness` - Shadow darkness (0.0 = no shadow, 1.0 = pure black)
   * * `blur_radius` - Blur radius in pixels (typically 1-3px for contact shadows)
   * * `offset_y` - Vertical offset in pixels (typically 0-1px)
   * * `spread` - Shadow spread (typically 0 for contact shadows)
   */
  constructor(darkness: number, blur_radius: number, offset_y: number, spread: number);
  /**
   * Create default contact shadow params (standard glass contact shadow).
   */
  static default(): ContactShadowParams;
  /**
   * Standard glass contact shadow preset.
   */
  static standard(): ContactShadowParams;
  /**
   * Floating glass preset (lighter contact shadow).
   */
  static floating(): ContactShadowParams;
  /**
   * Grounded glass preset (heavier contact shadow).
   */
  static grounded(): ContactShadowParams;
  /**
   * Subtle preset (barely visible contact shadow).
   */
  static subtle(): ContactShadowParams;
  readonly darkness: number;
  readonly blurRadius: number;
  readonly offsetY: number;
  readonly spread: number;
}

export class ContractBuilder {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  minContrastWcagAA(against_hex: string): ContractBuilder;
  minContrastWcagAAA(against_hex: string): ContractBuilder;
  inSrgb(): ContractBuilder;
  inP3(): ContractBuilder;
  lightnessRange(min: number, max: number): ContractBuilder;
  chromaRange(min: number, max: number): ContractBuilder;
  hueRange(min: number, max: number): ContractBuilder;
  build(): string;
  buildAndValidate(color_hex: string): string;
}

export class ContrastResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * The contrast value.
   *
   * Interpretation depends on metric:
   * - WCAG: 1.0 to 21.0 (contrast ratio)
   * - APCA: -108 to +106 (Lc value, signed)
   */
  value: number;
  /**
   * Polarity of the contrast (APCA only).
   *
   * - 1 = dark on light
   * - -1 = light on dark
   * - 0 = not applicable (WCAG)
   */
  polarity: number;
}

export class ConvergenceDetector {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create with default config.
   */
  constructor();
  /**
   * Create with "fast" preset (fewer iterations, lower threshold).
   */
  static fast(): ConvergenceDetector;
  /**
   * Create with "high_quality" preset (more iterations, tighter threshold).
   */
  static highQuality(): ConvergenceDetector;
  /**
   * Create with "neural" preset (optimized for neural correction loops).
   */
  static neural(): ConvergenceDetector;
  /**
   * Feed a new quality value. Returns status as JSON.
   */
  update(quality: number): any;
  /**
   * Reset the detector to initial state.
   */
  reset(): void;
  /**
   * Current best quality observed.
   */
  bestQuality(): number;
  /**
   * Total iterations so far.
   */
  iterationCount(): number;
  /**
   * Total quality improvement from start.
   */
  totalImprovement(): number;
  /**
   * Average improvement per iteration.
   */
  improvementRate(): number;
  /**
   * Whether quality is still improving.
   */
  isProgressing(): boolean;
  /**
   * Full stats as JSON.
   */
  stats(): any;
}

export class CostEstimator {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Estimate cost for a step type with given factors.
   */
  estimate(step_type: string, color_count: number, spectral: boolean, neural: boolean, material: boolean): any;
  /**
   * Estimate sequential cost for multiple steps.
   */
  estimateSequential(steps_json: any): any;
}

export class CssBackend {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new CSS backend
   */
  constructor();
  /**
   * Render evaluated material to CSS string
   *
   * # Arguments
   *
   * * `material` - Evaluated material with resolved properties
   * * `context` - Rendering context
   *
   * # Returns
   *
   * CSS string with all material properties, or error
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const glass = GlassMaterial.frosted();
   * const evalCtx = EvalMaterialContext.new();
   * const evaluated = glass.evaluate(evalCtx);
   *
   * const backend = new CssBackend();
   * const renderCtx = RenderContext.desktop();
   * const css = backend.render(evaluated, renderCtx);
   * console.log(css); // "backdrop-filter: blur(24px); background: ..."
   * ```
   */
  render(material: EvaluatedMaterial, context: RenderContext): string;
  /**
   * Get backend name
   */
  name(): string;
}

export class CssRenderConfig {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  static minimal(): CssRenderConfig;
  static premium(): CssRenderConfig;
  static modal(): CssRenderConfig;
  static subtle(): CssRenderConfig;
  static darkMode(): CssRenderConfig;
  withSpecularIntensity(intensity: number): CssRenderConfig;
  withFresnelIntensity(intensity: number): CssRenderConfig;
  withElevation(level: number): CssRenderConfig;
  withBorderRadius(radius: number): CssRenderConfig;
  withLightMode(light_mode: boolean): CssRenderConfig;
  withEffectsEnabled(enabled: boolean): CssRenderConfig;
  toJson(): any;
}

export class DielectricBSDF {
  free(): void;
  [Symbol.dispose](): void;
  constructor(ior: number, roughness: number);
  static glass(): DielectricBSDF;
  static water(): DielectricBSDF;
  static diamond(): DielectricBSDF;
  static frostedGlass(): DielectricBSDF;
  evaluate(wi_x: number, wi_y: number, wi_z: number, wo_x: number, wo_y: number, wo_z: number): any;
  validateEnergy(): any;
}

export class DrudeParams {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create custom Drude parameters.
   */
  constructor(eps_inf: number, omega_p: number, gamma: number, t_ref: number, d_omega_p: number, d_gamma: number);
  /**
   * Gold (Au) - Drude model from Ordal et al. (1983).
   */
  static gold(): DrudeParams;
  /**
   * Silver (Ag) - Drude model.
   */
  static silver(): DrudeParams;
  /**
   * Copper (Cu) - Drude model.
   */
  static copper(): DrudeParams;
  /**
   * Aluminum (Al) - Drude model.
   */
  static aluminum(): DrudeParams;
  /**
   * Iron (Fe) - Drude model.
   */
  static iron(): DrudeParams;
  /**
   * Platinum (Pt) - Drude model.
   */
  static platinum(): DrudeParams;
  /**
   * Nickel (Ni) - Drude model.
   */
  static nickel(): DrudeParams;
  /**
   * Calculate complex IOR at given wavelength and temperature.
   *
   * # Arguments
   *
   * * `wavelength_nm` - Wavelength in nanometers
   * * `temp_k` - Temperature in Kelvin
   *
   * # Returns
   *
   * ComplexIOR with temperature-adjusted n and k
   */
  complexIor(wavelength_nm: number, temp_k: number): ComplexIOR;
  /**
   * Calculate spectral IOR (RGB) at given temperature.
   */
  spectralIor(temp_k: number): SpectralComplexIOR;
  /**
   * Get temperature-adjusted plasma frequency and damping.
   *
   * # Returns
   *
   * Object { omegaP, gamma } at the given temperature
   */
  atTemperature(temp_k: number): object;
}

export class DynamicFilmLayer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new dynamic film layer
   *
   * # Arguments
   * * `n` - Base refractive index at reference temperature (293K)
   * * `thickness_nm` - Base thickness in nanometers
   *
   * Default properties:
   * - dn/dT = 10⁻⁵ K⁻¹ (typical glass)
   * - α_thermal = 5×10⁻⁶ K⁻¹ (typical SiO₂)
   * - Young's modulus = 70 GPa
   * - Poisson ratio = 0.17
   */
  constructor(n: number, thickness_nm: number);
  /**
   * Set thermo-optic coefficient dn/dT
   *
   * # Arguments
   * * `dn_dt` - Thermo-optic coefficient in K⁻¹
   *
   * Typical values:
   * - SiO₂: 1.0×10⁻⁵ K⁻¹
   * - BK7 glass: 2.3×10⁻⁶ K⁻¹
   * - Water: 1.0×10⁻⁴ K⁻¹
   * - Polycarbonate: -1.0×10⁻⁴ K⁻¹ (negative!)
   */
  withDnDt(dn_dt: number): DynamicFilmLayer;
  /**
   * Set thermal expansion coefficient α
   *
   * # Arguments
   * * `alpha` - Thermal expansion coefficient in K⁻¹
   *
   * Typical values:
   * - SiO₂: 5×10⁻⁷ K⁻¹
   * - BK7 glass: 7×10⁻⁶ K⁻¹
   * - Water (film): 2×10⁻⁴ K⁻¹
   * - Aluminum: 2.3×10⁻⁵ K⁻¹
   */
  withThermalExpansion(alpha: number): DynamicFilmLayer;
  /**
   * Set mechanical properties
   *
   * # Arguments
   * * `youngs_modulus` - Young's modulus in GPa
   * * `poisson_ratio` - Poisson's ratio (typically 0.1-0.4)
   */
  withMechanical(youngs_modulus: number, poisson_ratio: number): DynamicFilmLayer;
  /**
   * Set extinction coefficient k
   */
  withK(k: number): DynamicFilmLayer;
  /**
   * Set current temperature (K)
   *
   * # Arguments
   * * `temp_k` - Temperature in Kelvin
   *
   * Valid range: 100K to 1000K (outside range may give unphysical results)
   */
  setTemperature(temp_k: number): void;
  /**
   * Set stress state (Voigt notation)
   *
   * # Arguments
   * * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
   *
   * For uniaxial stress σ in z-direction: [0, 0, σ, 0, 0, 0]
   * For biaxial stress σ in xy-plane: [σ, σ, 0, 0, 0, 0]
   */
  setStress(stress: Float64Array): void;
  /**
   * Get all effective properties as [n, k, thickness]
   */
  effectiveProperties(): Float64Array;
  /**
   * Get effective refractive index at current temperature
   *
   * n_eff = n_base + dn/dT × (T - T_ref)
   */
  readonly effectiveN: number;
  /**
   * Get effective thickness at current conditions (temperature + stress)
   *
   * d_eff = d_base × (1 + thermal_strain + stress_strain)
   */
  readonly effectiveThickness: number;
  readonly nBase: number;
  readonly kBase: number;
  readonly baseThickness: number;
  readonly dnDt: number;
  readonly alphaThermal: number;
  readonly temperature: number;
}

export class DynamicMieParams {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create monodisperse (single-size) particle distribution.
   *
   * # Arguments
   *
   * * `radius_um` - Particle radius in micrometers
   * * `n_particle` - Particle refractive index
   * * `n_medium` - Medium refractive index
   */
  constructor(radius_um: number, n_particle: number, n_medium: number);
  /**
   * Create log-normal size distribution.
   *
   * Most realistic for atmospheric particles.
   *
   * # Arguments
   *
   * * `geometric_mean_um` - Geometric mean radius (µm)
   * * `geometric_std` - Geometric standard deviation (dimensionless, typically 1.2-2.5)
   * * `n_particle` - Particle refractive index
   * * `n_medium` - Medium refractive index
   */
  static logNormal(geometric_mean_um: number, geometric_std: number, n_particle: number, n_medium: number): DynamicMieParams;
  /**
   * Create bimodal size distribution.
   *
   * Useful for smoke (fine soot + coarse aggregates).
   *
   * # Arguments
   *
   * * `mean1_um`, `std1` - First mode parameters
   * * `mean2_um`, `std2` - Second mode parameters
   * * `weight1` - Weight of first mode (0-1)
   * * `n_particle`, `n_medium` - Refractive indices
   */
  static bimodal(mean1_um: number, std1: number, mean2_um: number, std2: number, weight1: number, n_particle: number, n_medium: number): DynamicMieParams;
  /**
   * Stratocumulus cloud droplets (~8µm water, forward scattering).
   */
  static stratocumulus(): DynamicMieParams;
  /**
   * Fog droplets (~4µm water).
   */
  static fog(): DynamicMieParams;
  /**
   * Smoke particles (bimodal soot distribution).
   */
  static smoke(): DynamicMieParams;
  /**
   * Milk (fat globules in water).
   */
  static milk(): DynamicMieParams;
  /**
   * Desert dust storm.
   */
  static dust(): DynamicMieParams;
  /**
   * Ice crystals (cirrus clouds).
   */
  static iceCrystals(): DynamicMieParams;
  /**
   * Condensing fog (growing droplets).
   */
  static condensingFog(): DynamicMieParams;
  /**
   * Evaporating mist (shrinking droplets).
   */
  static evaporatingMist(): DynamicMieParams;
  /**
   * Calculate polydisperse phase function.
   *
   * Integrates over the size distribution:
   * p_total(θ) = ∫ p(θ, r) × n(r) dr
   *
   * # Arguments
   *
   * * `cos_theta` - Cosine of scattering angle
   * * `wavelength_nm` - Wavelength in nanometers
   */
  phaseFunction(cos_theta: number, wavelength_nm: number): number;
  /**
   * Calculate RGB polydisperse phase function.
   *
   * Returns [p_red, p_green, p_blue] integrated over size distribution.
   */
  phaseRgb(cos_theta: number): Float64Array;
  /**
   * Calculate effective asymmetry parameter for the distribution.
   *
   * Weighted average of g over all particle sizes.
   */
  effectiveG(wavelength_nm: number): number;
  /**
   * Calculate extinction coefficient for the distribution.
   *
   * Total extinction from all particle sizes.
   */
  extinctionCoeff(wavelength_nm: number): number;
  /**
   * Generate CSS for fog-like volumetric effect.
   *
   * # Arguments
   *
   * * `density` - Optical density (0-1)
   */
  toCssFog(density: number): string;
  /**
   * Generate CSS for smoke-like volumetric effect.
   *
   * # Arguments
   *
   * * `density` - Optical density (0-1)
   */
  toCssSmoke(density: number): string;
}

export class DynamicThinFilmStack {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new empty dynamic stack
   *
   * # Arguments
   * * `n_ambient` - Ambient medium refractive index (1.0 for air)
   * * `n_substrate` - Substrate refractive index
   */
  constructor(n_ambient: number, n_substrate: number);
  /**
   * Add a dynamic layer to the stack
   */
  addLayer(layer: DynamicFilmLayer): void;
  /**
   * Set environmental conditions
   *
   * # Arguments
   * * `temp_k` - Temperature in Kelvin
   * * `pressure_pa` - Pressure in Pascals (standard: 101325 Pa)
   * * `humidity` - Relative humidity (0.0 to 1.0)
   *
   * This updates ALL layers in the stack to the new temperature.
   */
  setEnvironment(temp_k: number, pressure_pa: number, humidity: number): void;
  /**
   * Apply uniform stress to all layers
   *
   * # Arguments
   * * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
   */
  applyStress(stress: Float64Array): void;
  /**
   * Calculate reflectance at a surface position
   *
   * # Arguments
   * * `pos_x` - Normalized x position (0.0 to 1.0)
   * * `pos_y` - Normalized y position (0.0 to 1.0)
   * * `wavelength_nm` - Wavelength in nanometers
   * * `angle_deg` - Viewing angle in degrees
   *
   * # Returns
   * Reflectance (0.0 to 1.0)
   */
  reflectanceAt(pos_x: number, pos_y: number, wavelength_nm: number, angle_deg: number): number;
  /**
   * Calculate RGB reflectance at a surface position
   *
   * # Arguments
   * * `pos_x` - Normalized x position (0.0 to 1.0)
   * * `pos_y` - Normalized y position (0.0 to 1.0)
   * * `angle_deg` - Viewing angle in degrees
   *
   * # Returns
   * [R, G, B] reflectance array (0.0 to 1.0)
   */
  reflectanceRgbAt(pos_x: number, pos_y: number, angle_deg: number): Float64Array;
  /**
   * Calculate RGB reflectance at center of surface (convenience method)
   *
   * # Arguments
   * * `angle_deg` - Viewing angle in degrees
   *
   * # Returns
   * [R, G, B] reflectance array (0.0 to 1.0)
   */
  reflectanceRgb(angle_deg: number): Float64Array;
  /**
   * Get total optical thickness of the stack at current conditions
   *
   * # Returns
   * Total thickness in nanometers (sum of all layer thicknesses)
   */
  totalThickness(): number;
  /**
   * Create a soap bubble with temperature response
   *
   * Water film (n=1.33) has high dn/dT (~10⁻⁴ K⁻¹) and
   * significant thermal expansion (~2×10⁻⁴ K⁻¹).
   *
   * # Arguments
   * * `temp_k` - Initial temperature in Kelvin
   */
  static soapBubble(temp_k: number): DynamicThinFilmStack;
  /**
   * Create an AR coating with stress response
   *
   * MgF₂ coating on glass with mechanical properties.
   * Stress affects thickness and therefore optical performance.
   *
   * # Arguments
   * * `stress_mpa` - Applied biaxial stress in MPa
   */
  static arCoatingStressed(stress_mpa: number): DynamicThinFilmStack;
  /**
   * Create an oil slick on water with ripple pattern
   */
  static oilSlickRippled(): DynamicThinFilmStack;
  readonly ambientTemp: number;
  readonly ambientPressure: number;
  readonly humidity: number;
  readonly layerCount: number;
}

/**
 * Material Design 3 elevation levels.
 */
export enum Elevation {
  /**
   * Level 0 - Base surface
   */
  Level0 = 0,
  /**
   * Level 1 - 1dp elevation
   */
  Level1 = 1,
  /**
   * Level 2 - 3dp elevation
   */
  Level2 = 2,
  /**
   * Level 3 - 6dp elevation
   */
  Level3 = 3,
  /**
   * Level 4 - 8dp elevation
   */
  Level4 = 4,
  /**
   * Level 5 - 12dp elevation
   */
  Level5 = 5,
}

export class ElevationPresets {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Flush with surface (no elevation)
   */
  static readonly LEVEL_0: number;
  /**
   * Subtle lift (standard buttons)
   */
  static readonly LEVEL_1: number;
  /**
   * Hover state (interactive lift)
   */
  static readonly LEVEL_2: number;
  /**
   * Floating cards
   */
  static readonly LEVEL_3: number;
  /**
   * Modals, sheets
   */
  static readonly LEVEL_4: number;
  /**
   * Dropdowns, tooltips
   */
  static readonly LEVEL_5: number;
  /**
   * Drag state (maximum separation)
   */
  static readonly LEVEL_6: number;
}

export class ElevationShadow {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Convert to CSS box-shadow string
   */
  toCSS(): string;
  /**
   * Get elevation level used
   */
  readonly elevation: number;
}

export class ElevationTransition {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create from elevation dp values (raw u8).
   */
  constructor(rest: number, hover: number, active: number, focus: number);
  static card(): ElevationTransition;
  static fab(): ElevationTransition;
  static flat(): ElevationTransition;
}

export class EvalMaterialContext {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create default evaluation context
   *
   * Uses standard viewing angle (0° = looking straight at surface),
   * neutral background, and default lighting.
   */
  constructor();
  /**
   * Create context with custom background color
   */
  static withBackground(background: OKLCH): EvalMaterialContext;
  /**
   * Create context with custom viewing angle
   *
   * # Arguments
   *
   * * `angle_deg` - Viewing angle in degrees (0° = perpendicular, 90° = edge-on)
   */
  static withViewingAngle(angle_deg: number): EvalMaterialContext;
  /**
   * Get background color
   */
  readonly background: OKLCH;
  /**
   * Get viewing angle in degrees
   */
  readonly viewingAngle: number;
  /**
   * Get ambient light intensity
   */
  readonly ambientLight: number;
  /**
   * Get key light intensity
   */
  readonly keyLight: number;
}

export class EvaluatedMaterial {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get base color (RGB in linear space)
   */
  baseColor(): Float64Array;
  /**
   * Get final opacity (0.0-1.0)
   */
  readonly opacity: number;
  /**
   * Get Fresnel reflectance at normal incidence (F0)
   */
  readonly fresnelF0: number;
  /**
   * Get edge intensity for Fresnel glow
   */
  readonly fresnelEdgeIntensity: number;
  /**
   * Get index of refraction (if applicable)
   */
  readonly ior: number | undefined;
  /**
   * Get surface roughness (0.0-1.0)
   */
  readonly roughness: number;
  /**
   * Get scattering radius in millimeters (physical property)
   */
  readonly scatteringRadiusMm: number;
  /**
   * Get blur amount in CSS pixels (DEPRECATED)
   *
   * **DEPRECATED:** Use scatteringRadiusMm instead and convert in your renderer.
   * This method assumes 96 DPI and will be removed in v6.0.
   */
  readonly blurPx: number;
  /**
   * Get specular intensity
   */
  readonly specularIntensity: number;
  /**
   * Get specular shininess
   */
  readonly specularShininess: number;
  /**
   * Get thickness in millimeters
   */
  readonly thicknessMm: number;
  /**
   * Get absorption coefficients (RGB)
   */
  readonly absorption: Float64Array;
  /**
   * Get scattering coefficients (RGB)
   */
  readonly scattering: Float64Array;
}

export class EvaluationContext {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create default context (normal incidence, room temperature)
   */
  constructor();
  /**
   * Set viewing angle in degrees
   *
   * # Arguments
   * * `angle_deg` - Angle from normal in degrees (0 = normal, 90 = grazing)
   */
  withAngle(angle_deg: number): EvaluationContext;
  /**
   * Set temperature in Kelvin
   *
   * # Arguments
   * * `temp_k` - Temperature in Kelvin
   */
  withTemperature(temp_k: number): EvaluationContext;
  /**
   * Set stress tensor
   *
   * # Arguments
   * * `stress` - [σxx, σyy, σzz, σxy, σyz, σzx] in MPa
   */
  withStress(stress: Float64Array): EvaluationContext;
  /**
   * Set surface position
   *
   * # Arguments
   * * `x` - X position (0.0 to 1.0)
   * * `y` - Y position (0.0 to 1.0)
   */
  withPosition(x: number, y: number): EvaluationContext;
  readonly cosTheta: number;
  readonly temperatureK: number;
}

export class ExplanationGenerator {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Generate an explanation for a contrast improvement recommendation.
   */
  explainContrastImprovement(original_hex: string, recommended_hex: string, background_hex: string, original_ratio: number, new_ratio: number, target_ratio: number, delta_l: number, delta_c: number, delta_h: number): RecommendationExplanation;
  /**
   * Generate an explanation for a quality score improvement.
   */
  explainQualityImprovement(original_hex: string, recommended_hex: string, before_overall: number, before_compliance: number, before_perceptual: number, before_appropriateness: number, after_overall: number, after_compliance: number, after_perceptual: number, after_appropriateness: number, delta_l: number, delta_c: number, delta_h: number): RecommendationExplanation;
}

export class ExponentialMovingAverage {
  free(): void;
  [Symbol.dispose](): void;
  constructor(alpha: number);
  update(value: number): number;
  reset(): void;
  readonly value: number;
}

export class FilmLayer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a dielectric (lossless) layer
   *
   * # Arguments
   * * `n` - Real refractive index
   * * `thickness_nm` - Layer thickness in nanometers
   *
   * # Example
   * ```javascript
   * // Quarter-wave MgF2 layer at 550nm
   * const layer = FilmLayer.dielectric(1.38, 99.6);
   * ```
   */
  constructor(n: number, thickness_nm: number);
  /**
   * Create an absorbing layer with complex IOR (n + ik)
   *
   * # Arguments
   * * `n` - Real part of refractive index
   * * `k` - Extinction coefficient (imaginary part)
   * * `thickness_nm` - Layer thickness in nanometers
   *
   * # Example
   * ```javascript
   * // Thin aluminum layer
   * const al = FilmLayer.absorbing(0.15, 3.5, 50.0);
   * ```
   */
  static absorbing(n: number, k: number, thickness_nm: number): FilmLayer;
  /**
   * Get real part of refractive index
   */
  readonly n: number;
  /**
   * Get extinction coefficient (imaginary part of n)
   */
  readonly k: number;
  /**
   * Get layer thickness in nanometers
   */
  readonly thicknessNm: number;
}

export class FlickerValidator {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  static strict(): FlickerValidator;
  static relaxed(): FlickerValidator;
  static withThresholds(stable: number, minor: number, warning: number): FlickerValidator;
}

export class Gamma {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Convert sRGB channel value (0.0-1.0) to linear RGB.
   *
   * # Arguments
   * * `channel` - sRGB channel value (0.0 to 1.0)
   *
   * # Returns
   * Linear RGB channel value
   *
   * # Example (JavaScript)
   * ```javascript
   * const srgb = 0.5; // Mid gray in sRGB
   * const linear = Gamma.srgbToLinear(srgb);
   * console.log(linear); // ~0.214 (NOT 0.5!)
   * ```
   */
  static srgbToLinear(channel: number): number;
  /**
   * Convert linear RGB channel value (0.0-1.0) to sRGB.
   *
   * # Arguments
   * * `channel` - Linear RGB channel value (0.0 to 1.0)
   *
   * # Returns
   * sRGB channel value
   *
   * # Example (JavaScript)
   * ```javascript
   * const linear = 0.214;
   * const srgb = Gamma.linearToSrgb(linear);
   * console.log(srgb); // ~0.5
   * ```
   */
  static linearToSrgb(channel: number): number;
  /**
   * Convert RGB array from sRGB to linear.
   *
   * # Arguments
   * * `r`, `g`, `b` - sRGB values (0.0 to 1.0)
   *
   * # Returns
   * Linear RGB values as Float64Array
   */
  static rgbToLinear(r: number, g: number, b: number): Float64Array;
  /**
   * Convert RGB array from linear to sRGB.
   *
   * # Arguments
   * * `r`, `g`, `b` - Linear RGB values (0.0 to 1.0)
   *
   * # Returns
   * sRGB values as Float64Array
   */
  static linearToRgb(r: number, g: number, b: number): Float64Array;
}

export class GamutUtils {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Estimate maximum chroma for given lightness and hue.
   *
   * Uses parabolic approximation for fast gamut boundary estimation.
   *
   * # Arguments
   * * `l` - Lightness (0.0 to 1.0)
   * * `h` - Hue (0.0 to 360.0 degrees)
   *
   * # Returns
   * Estimated maximum chroma that stays within sRGB gamut
   *
   * # Example (JavaScript)
   * ```javascript
   * const maxChroma = GamutUtils.estimateMaxChroma(0.5, 180.0); // Cyan at mid-L
   * console.log(maxChroma); // ~0.06
   * ```
   */
  static estimateMaxChroma(l: number, h: number): number;
  /**
   * Check if OKLCH color is approximately within sRGB gamut.
   *
   * # Arguments
   * * `l` - Lightness (0.0 to 1.0)
   * * `c` - Chroma
   * * `h` - Hue (0.0 to 360.0 degrees)
   *
   * # Returns
   * true if color is within sRGB gamut (with 10% tolerance)
   */
  static isInGamut(l: number, c: number, h: number): boolean;
  /**
   * Map OKLCH color to sRGB gamut by reducing chroma.
   *
   * Preserves lightness and hue while finding maximum achievable chroma.
   *
   * # Arguments
   * * `l` - Lightness (0.0 to 1.0)
   * * `c` - Chroma
   * * `h` - Hue (0.0 to 360.0 degrees)
   *
   * # Returns
   * OKLCH color with chroma reduced to fit within sRGB gamut
   */
  static mapToGamut(l: number, c: number, h: number): OKLCH;
}

export class GlassLayers {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Top layer: Specular highlights
   */
  readonly highlight: OKLCH;
  /**
   * Middle layer: Base glass tint
   */
  readonly base: OKLCH;
  /**
   * Bottom layer: Shadow for depth
   */
  readonly shadow: OKLCH;
}

export class GlassMaterial {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create glass material with custom properties
   *
   * # Arguments
   *
   * * `ior` - Index of refraction (1.0-2.5, typical glass: 1.5)
   * * `roughness` - Surface roughness (0.0-1.0, 0 = mirror-smooth)
   * * `thickness` - Thickness in millimeters
   * * `noise_scale` - Frosted texture amount (0.0-1.0)
   * * `base_color` - Material tint color
   * * `edge_power` - Fresnel edge sharpness (1.0-4.0)
   */
  constructor(ior: number, roughness: number, thickness: number, noise_scale: number, base_color: OKLCH, edge_power: number);
  /**
   * Create clear glass preset
   * IOR: 1.5, Roughness: 0.05, Thickness: 2mm
   */
  static clear(): GlassMaterial;
  /**
   * Create regular glass preset (Apple-like)
   * IOR: 1.5, Roughness: 0.15, Thickness: 5mm
   */
  static regular(): GlassMaterial;
  /**
   * Create thick glass preset
   * IOR: 1.52, Roughness: 0.25, Thickness: 10mm
   */
  static thick(): GlassMaterial;
  /**
   * Create frosted glass preset
   * IOR: 1.5, Roughness: 0.6, Thickness: 8mm
   */
  static frosted(): GlassMaterial;
  /**
   * Calculate Blinn-Phong shininess from roughness
   */
  shininess(): number;
  /**
   * Calculate scattering radius in millimeters (physical property)
   */
  scatteringRadiusMm(): number;
  /**
   * Calculate translucency (opacity 0-1)
   */
  translucency(): number;
  /**
   * Evaluate material properties based on context (Phase 3 pipeline)
   *
   * Performs full physics-based evaluation including Fresnel reflectance,
   * Beer-Lambert absorption, and subsurface scattering.
   *
   * # Arguments
   *
   * * `context` - Material evaluation context (lighting, viewing angle, background)
   *
   * # Returns
   *
   * EvaluatedMaterial with all optical properties resolved
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const glass = GlassMaterial.frosted();
   * const context = EvalMaterialContext.default();
   * const evaluated = glass.evaluate(context);
   * console.log(`Opacity: ${evaluated.opacity}`);
   * console.log(`Scattering: ${evaluated.scatteringRadiusMm}mm`);
   * ```
   */
  evaluate(context: EvalMaterialContext): EvaluatedMaterial;
  /**
   * Create a builder for custom glass materials (Gap 5 - P1).
   *
   * Provides a fluent API for creating glass materials with custom properties.
   * Unset properties default to the "regular" preset values.
   *
   * # Example (JavaScript)
   *
   * ```javascript
   * const custom = GlassMaterial.builder()
   *     .ior(1.45)
   *     .roughness(0.3)
   *     .thickness(8.0)
   *     .build();
   * ```
   */
  static builder(): GlassMaterialBuilder;
  /**
   * Get index of refraction
   */
  readonly ior: number;
  /**
   * Get surface roughness
   */
  readonly roughness: number;
  /**
   * Get thickness in millimeters
   */
  readonly thickness: number;
  /**
   * Get noise scale
   */
  readonly noiseScale: number;
  /**
   * Get base color
   */
  readonly baseColor: OKLCH;
  /**
   * Get edge power
   */
  readonly edgePower: number;
}

export class GlassMaterialBuilder {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new builder with no preset values.
   */
  constructor();
  /**
   * Start from the "clear" preset.
   */
  presetClear(): GlassMaterialBuilder;
  /**
   * Start from the "regular" preset.
   */
  presetRegular(): GlassMaterialBuilder;
  /**
   * Start from the "thick" preset.
   */
  presetThick(): GlassMaterialBuilder;
  /**
   * Start from the "frosted" preset.
   */
  presetFrosted(): GlassMaterialBuilder;
  /**
   * Set the index of refraction (IOR).
   *
   * Valid range: 1.0 - 2.5
   */
  ior(ior: number): GlassMaterialBuilder;
  /**
   * Set the surface roughness.
   *
   * Valid range: 0.0 - 1.0
   */
  roughness(roughness: number): GlassMaterialBuilder;
  /**
   * Set the glass thickness in millimeters.
   */
  thickness(mm: number): GlassMaterialBuilder;
  /**
   * Set the noise scale for frosted texture.
   *
   * Valid range: 0.0 - 1.0
   */
  noiseScale(scale: number): GlassMaterialBuilder;
  /**
   * Set the base color tint.
   */
  baseColor(color: OKLCH): GlassMaterialBuilder;
  /**
   * Set the edge power for Fresnel glow.
   *
   * Valid range: 1.0 - 4.0
   */
  edgePower(power: number): GlassMaterialBuilder;
  /**
   * Build the GlassMaterial.
   *
   * Any unset properties default to the "regular" preset values.
   */
  build(): GlassMaterial;
}

export class GlassPhysicsEngine {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new glass physics engine with material preset
   *
   * # Arguments
   *
   * * `preset` - "clear", "regular", "thick", or "frosted"
   */
  constructor(preset: string);
  /**
   * Create with custom material and noise
   */
  static withCustom(material: GlassMaterial, noise: PerlinNoise): GlassPhysicsEngine;
  /**
   * Calculate complete glass properties for rendering
   *
   * Returns object with all CSS-ready values:
   * - opacity: Material translucency (0-1)
   * - blur: Blur amount in pixels
   * - fresnel: Array of gradient stops [position, intensity, ...]
   * - specular: Array of layer data [intensity, x, y, size, ...]
   * - noise: Noise texture scale
   */
  calculateProperties(normal: Vec3, light_dir: Vec3, view_dir: Vec3): object;
  /**
   * Generate noise texture
   */
  generateNoiseTexture(width: number, height: number, scale: number): Uint8Array;
  /**
   * Get material
   */
  readonly material: GlassMaterial;
}

export class GlassProperties {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create default glass properties.
   */
  constructor();
  /**
   * Get base tint color.
   */
  getBaseTint(): OKLCH;
  /**
   * Set base tint color.
   */
  setBaseTint(tint: OKLCH): void;
  /**
   * Get opacity (0.0 = transparent, 1.0 = opaque).
   */
  opacity: number;
  /**
   * Get blur radius in pixels.
   */
  blurRadius: number;
  /**
   * Get reflectivity (0.0 = none, 1.0 = mirror).
   */
  reflectivity: number;
  /**
   * Get refraction index.
   */
  refraction: number;
  /**
   * Get depth/thickness.
   */
  depth: number;
  /**
   * Get noise scale.
   */
  noiseScale: number;
  /**
   * Get specular intensity.
   */
  specularIntensity: number;
}

export class GlassRenderOptions {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create options with default settings.
   */
  constructor();
  /**
   * Create minimal preset (no visual effects).
   */
  static minimal(): GlassRenderOptions;
  /**
   * Create premium preset (Apple Liquid Glass quality).
   */
  static premium(): GlassRenderOptions;
  /**
   * Create modal preset (floating dialogs).
   */
  static modal(): GlassRenderOptions;
  /**
   * Create subtle preset (content-focused cards).
   */
  static subtle(): GlassRenderOptions;
  /**
   * Create dark mode preset.
   */
  static darkMode(): GlassRenderOptions;
  /**
   * Enable or disable specular highlights.
   */
  set specularEnabled(value: boolean);
  /**
   * Set specular highlight intensity (0.0-1.0).
   */
  set specularIntensity(value: number);
  /**
   * Enable or disable Fresnel edge glow.
   */
  set fresnelEnabled(value: boolean);
  /**
   * Set Fresnel edge intensity (0.0-1.0).
   */
  set fresnelIntensity(value: number);
  /**
   * Set elevation level (0-6).
   */
  set elevation(value: number);
  /**
   * Enable or disable backdrop saturation boost.
   */
  set saturate(value: boolean);
  /**
   * Set border radius in pixels.
   */
  set borderRadius(value: number);
  /**
   * Set light mode (true) or dark mode (false).
   */
  set lightMode(value: boolean);
  /**
   * Enable or disable inner highlight.
   */
  set innerHighlightEnabled(value: boolean);
  /**
   * Enable or disable border.
   */
  set borderEnabled(value: boolean);
}

/**
 * Glass variant defines the visual behavior of Liquid Glass.
 */
export enum GlassVariant {
  /**
   * Regular glass - adaptive, most versatile
   */
  Regular = 0,
  /**
   * Clear glass - permanently more transparent
   */
  Clear = 1,
}

export class HCT {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create an HCT color from hue, chroma, and tone.
   */
  constructor(hue: number, chroma: number, tone: number);
  /**
   * Convert an sRGB hex string to HCT.
   *
   * # Arguments
   * * `hex` — hex color string (e.g. "#3a7bd5" or "3a7bd5")
   *
   * # Returns
   * HCT instance, or HCT(0, 0, 0) if hex is invalid.
   */
  static fromHex(hex: string): HCT;
  /**
   * Convert from ARGB integer (0xAARRGGBB).
   */
  static fromArgb(argb: number): HCT;
  /**
   * Convert HCT to an ARGB integer (0xFF_RR_GG_BB).
   */
  toArgb(): number;
  /**
   * Convert HCT to a hex color string (e.g. "#3a7bd5").
   */
  toHex(): string;
  /**
   * Convert HCT to OKLCH flat array `[L, C, H]`.
   */
  toOklch(): Float64Array;
  /**
   * Clone with a different tone (preserves hue and chroma).
   */
  withTone(tone: number): HCT;
  /**
   * Clone with a different chroma (preserves hue and tone).
   */
  withChroma(chroma: number): HCT;
  /**
   * Clone with a different hue (preserves chroma and tone).
   */
  withHue(hue: number): HCT;
  /**
   * Clamp chroma to the maximum achievable in the sRGB gamut.
   */
  clampToGamut(): HCT;
  /**
   * CAM16 hue angle in degrees (0–360°).
   */
  readonly hue: number;
  /**
   * CAM16 chroma (non-negative; maximum varies with tone and hue).
   */
  readonly chroma: number;
  /**
   * CIELAB L* tone (0 = black, 100 = white).
   */
  readonly tone: number;
}

/**
 * Interpolation mode enum for JS.
 */
export enum InterpolationModeEnum {
  Linear = 0,
  Smoothstep = 1,
  Smootherstep = 2,
  EaseInOut = 3,
  Step = 4,
}

export class LambertianBSDF {
  free(): void;
  [Symbol.dispose](): void;
  constructor(albedo: number);
  static white(): LambertianBSDF;
  static gray(): LambertianBSDF;
  evaluate(wi_x: number, wi_y: number, wi_z: number, wo_x: number, wo_y: number, wo_z: number): any;
}

export class LayerTransmittance {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Surface layer (edge highlight) - High reflectivity, bright
   */
  readonly surface: number;
  /**
   * Volume layer (glass body) - Main transmittance value
   */
  readonly volume: number;
  /**
   * Substrate layer (deep contact) - Darkest layer, creates depth
   */
  readonly substrate: number;
}

export class LayeredBSDF {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Add a dielectric layer.
   */
  pushDielectric(ior: number, roughness: number): LayeredBSDF;
  /**
   * Add a conductor layer.
   */
  pushConductor(n: number, k: number, roughness: number): LayeredBSDF;
  /**
   * Add a thin film layer.
   */
  pushThinFilm(substrate_ior: number, film_ior: number, thickness: number): LayeredBSDF;
  /**
   * Add a lambertian layer.
   */
  pushLambertian(albedo: number): LayeredBSDF;
  layerCount(): number;
  evaluate(wi_x: number, wi_y: number, wi_z: number, wo_x: number, wo_y: number, wo_z: number): any;
  validateEnergy(): any;
}

export class LightSource {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a light source. Color is specified as OKLCH (l, c, h).
   */
  constructor(dir_x: number, dir_y: number, dir_z: number, intensity: number, color_l: number, color_c: number, color_h: number);
  static defaultKeyLight(): LightSource;
  static defaultFillLight(): LightSource;
  static dramaticTopLight(): LightSource;
}

export class LightingContext {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create studio lighting preset
   */
  static studio(): LightingContext;
  /**
   * Create outdoor lighting preset
   */
  static outdoor(): LightingContext;
  /**
   * Create dramatic lighting preset
   */
  static dramatic(): LightingContext;
  /**
   * Create soft lighting preset
   */
  static soft(): LightingContext;
  /**
   * Create neutral lighting preset
   */
  static neutral(): LightingContext;
}

export class LightingEnvironment {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
}

export class LinearRgba {
  free(): void;
  [Symbol.dispose](): void;
  constructor(r: number, g: number, b: number, a: number);
  /**
   * Create from OKLCH color with alpha.
   */
  static fromOklch(oklch: OKLCH, alpha: number): LinearRgba;
  /**
   * Create opaque from linear RGB.
   */
  static rgb(r: number, g: number, b: number): LinearRgba;
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

export class LiquidGlass {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new Liquid Glass with specified variant.
   */
  constructor(variant: GlassVariant);
  /**
   * Create with custom properties.
   */
  static withProperties(variant: GlassVariant, properties: GlassProperties): LiquidGlass;
  /**
   * Calculate effective color when glass is over background.
   */
  effectiveColor(background: Color): Color;
  /**
   * Recommend text color for maximum readability.
   *
   * # Arguments
   *
   * * `background` - Background color behind the glass
   * * `prefer_white` - Whether to prefer white text over dark text
   */
  recommendTextColor(background: Color, prefer_white: boolean): Color;
  /**
   * Decompose into multi-layer structure.
   */
  getLayers(background: Color): GlassLayers;
  /**
   * Adapt glass properties for dark mode.
   */
  adaptForDarkMode(): void;
  /**
   * Adapt glass properties for light mode.
   */
  adaptForLightMode(): void;
  /**
   * Get variant.
   */
  readonly variant: GlassVariant;
  /**
   * Get properties.
   */
  readonly properties: GlassProperties;
}

export class LuminanceUtils {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Calculate relative luminance using WCAG/sRGB coefficients.
   *
   * Uses ITU-R BT.709 coefficients: 0.2126 R + 0.7152 G + 0.0722 B
   *
   * # Arguments
   * * `color` - The color to calculate luminance for
   *
   * # Returns
   * Relative luminance (0.0 to 1.0)
   */
  static relativeLuminanceSrgb(color: Color): number;
  /**
   * Calculate relative luminance using APCA coefficients.
   *
   * Uses APCA-specific coefficients for better perceptual accuracy.
   *
   * # Arguments
   * * `color` - The color to calculate luminance for
   *
   * # Returns
   * Relative luminance (0.0 to 1.0)
   */
  static relativeLuminanceApca(color: Color): number;
}

export class MaterialContext {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create studio preset context
   */
  static studio(): MaterialContext;
  /**
   * Create outdoor preset context
   */
  static outdoor(): MaterialContext;
  /**
   * Create dramatic preset context
   */
  static dramatic(): MaterialContext;
  /**
   * Create neutral preset context
   */
  static neutral(): MaterialContext;
  /**
   * Create showcase preset context
   */
  static showcase(): MaterialContext;
}

export class MaterialSurface {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create material surface from elevation and theme color.
   */
  constructor(elevation: Elevation, theme_primary: OKLCH);
  /**
   * Apply glass overlay to elevated surface.
   */
  withGlass(glass: LiquidGlass): MaterialSurface;
  /**
   * Calculate final surface color over base.
   */
  surfaceColor(base_surface: Color): Color;
  /**
   * Get elevation.
   */
  readonly elevation: Elevation;
  /**
   * Get surface tint.
   */
  readonly surfaceTint: OKLCH;
}

export enum MaterialTypeEnum {
  Glass = 0,
  Metal = 1,
  Plastic = 2,
  Liquid = 3,
  Custom = 4,
}

export class MieParams {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new Mie parameters.
   *
   * # Arguments
   *
   * * `radius_um` - Particle radius in micrometers
   * * `n_particle` - Particle refractive index
   * * `n_medium` - Medium refractive index (default 1.0 for air)
   */
  constructor(radius_um: number, n_particle: number, n_medium: number);
  /**
   * Fine dust (Rayleigh regime, x ~ 0.3).
   * Creates blue-ish scattering, responsible for blue sky.
   */
  static fineDust(): MieParams;
  /**
   * Coarse dust (Mie regime).
   */
  static coarseDust(): MieParams;
  /**
   * Small fog droplet (~2µm water).
   */
  static fogSmall(): MieParams;
  /**
   * Large fog droplet (~10µm water).
   */
  static fogLarge(): MieParams;
  /**
   * Cloud droplet (~8µm water).
   */
  static cloud(): MieParams;
  /**
   * Fine mist (~3µm water).
   */
  static mist(): MieParams;
  /**
   * Smoke particle (~0.3µm soot).
   */
  static smoke(): MieParams;
  /**
   * Milk fat globule (~2.5µm in water medium).
   */
  static milkGlobule(): MieParams;
  /**
   * Pollen grain (~25µm, geometric regime).
   */
  static pollen(): MieParams;
  /**
   * Calculate size parameter for a wavelength.
   *
   * x = 2πr/λ
   *
   * # Arguments
   *
   * * `wavelength_nm` - Wavelength in nanometers
   *
   * # Returns
   *
   * Size parameter x (dimensionless)
   */
  sizeParameter(wavelength_nm: number): number;
  /**
   * Calculate relative refractive index (m = n_particle / n_medium).
   */
  relativeIor(): number;
  /**
   * Size parameters at R/G/B wavelengths (650/550/450nm).
   */
  sizeParamRgb(): Float64Array;
  /**
   * Calculate Mie phase function at given angle and wavelength.
   *
   * # Arguments
   *
   * * `cos_theta` - Cosine of scattering angle (-1 to 1)
   * * `wavelength_nm` - Wavelength in nanometers
   *
   * # Returns
   *
   * Phase function value (probability density)
   */
  phaseFunction(cos_theta: number, wavelength_nm: number): number;
  /**
   * Calculate RGB phase function (wavelength-dependent).
   *
   * Returns [p_red, p_green, p_blue] at 650/550/450nm.
   */
  phaseRgb(cos_theta: number): Float64Array;
  /**
   * Calculate asymmetry parameter g.
   *
   * g = 0: Isotropic (Rayleigh)
   * g > 0: Forward scattering (Mie)
   * g ~ 0.85: Strong forward (clouds)
   */
  asymmetryG(wavelength_nm: number): number;
  /**
   * Calculate scattering and extinction efficiencies.
   *
   * # Returns
   *
   * Object { Qsca, Qext } efficiency factors
   */
  efficiencies(wavelength_nm: number): object;
  /**
   * Calculate scattering coefficient (1/µm).
   *
   * σ_s = Q_sca × π × r²
   */
  scatteringCoeff(wavelength_nm: number): number;
  /**
   * Calculate extinction coefficient (1/µm).
   *
   * σ_ext = Q_ext × π × r²
   */
  extinctionCoeff(wavelength_nm: number): number;
  /**
   * Particle radius in micrometers.
   */
  readonly radiusUm: number;
  /**
   * Particle refractive index.
   */
  readonly nParticle: number;
  /**
   * Medium refractive index.
   */
  readonly nMedium: number;
}

export class MomotoEventBus {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new event bus with default configuration.
   */
  constructor();
  /**
   * Create with custom buffer size and max age.
   */
  static withConfig(buffer_size: number, buffer_max_age_ms: bigint): MomotoEventBus;
  /**
   * Subscribe with a JS callback. Returns subscriber ID for unsubscribing.
   */
  subscribe(callback: Function): bigint;
  /**
   * Subscribe with category filter.
   */
  subscribeFiltered(categories: Uint8Array, callback: Function): bigint;
  /**
   * Emit a progress event.
   */
  emitProgress(source: string, progress: number, message: string): void;
  /**
   * Emit a metric event.
   */
  emitMetric(source: string, name: string, value: number): void;
  /**
   * Emit an error event.
   */
  emitError(source: string, description: string): void;
  /**
   * Emit a custom event from JSON payload.
   */
  emitCustom(source: string, payload_json: string): void;
  /**
   * Emit a full event from JSON.
   */
  emitJson(event_json: string): void;
  /**
   * Get the current subscriber count.
   */
  subscriberCount(): number;
  /**
   * Get the total event count emitted.
   */
  eventCount(): bigint;
  /**
   * Get all buffered events as JSON array.
   */
  bufferedEvents(): string;
  /**
   * Clear the event buffer.
   */
  clearBuffer(): void;
}

export class MomotoEventStream {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a real-time stream from an event bus.
   */
  static fromBus(bus: MomotoEventBus): MomotoEventStream;
  /**
   * Create a batched stream (more efficient for high-throughput).
   */
  static fromBusBatched(bus: MomotoEventBus, batch_size: number, timeout_ms: bigint): MomotoEventStream;
  /**
   * Create a standalone stream (no bus, manual push).
   */
  static standalone(): MomotoEventStream;
  /**
   * Push an event into the stream (for standalone streams).
   */
  push(event_json: string): void;
  /**
   * Poll for available events. Returns JSON or null.
   */
  poll(): any;
  /**
   * Force flush pending events.
   */
  flush(): any;
  /**
   * Check if flush should happen.
   */
  shouldFlush(): boolean;
  /**
   * Number of pending events.
   */
  pendingCount(): number;
  /**
   * Total events processed.
   */
  totalEvents(): bigint;
  /**
   * Total events dropped.
   */
  droppedEvents(): bigint;
  pause(): void;
  resume(): void;
  close(): void;
  /**
   * Get stream stats as JSON.
   */
  stats(): string;
  /**
   * Current stream state.
   */
  readonly state: string;
}

export class OKLCH {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create OKLCH color from L, C, H values.
   *
   * # Arguments
   *
   * * `l` - Lightness (0.0 to 1.0)
   * * `c` - Chroma (0.0 to ~0.4)
   * * `h` - Hue (0.0 to 360.0 degrees)
   */
  constructor(l: number, c: number, h: number);
  /**
   * Convert RGB color to OKLCH.
   */
  static fromColor(color: Color): OKLCH;
  /**
   * Convert OKLCH to RGB color.
   */
  toColor(): Color;
  /**
   * Make color lighter by delta.
   */
  lighten(delta: number): OKLCH;
  /**
   * Make color darker by delta.
   */
  darken(delta: number): OKLCH;
  /**
   * Increase chroma (saturation) by factor.
   */
  saturate(factor: number): OKLCH;
  /**
   * Decrease chroma (saturation) by factor.
   */
  desaturate(factor: number): OKLCH;
  /**
   * Rotate hue by degrees.
   */
  rotateHue(degrees: number): OKLCH;
  /**
   * Map to sRGB gamut by reducing chroma if necessary.
   */
  mapToGamut(): OKLCH;
  /**
   * Calculate perceptual difference (Delta E) between two colors.
   */
  deltaE(other: OKLCH): number;
  /**
   * Interpolate between two OKLCH colors.
   *
   * # Arguments
   *
   * * `a` - Start color
   * * `b` - End color
   * * `t` - Interpolation factor (0.0 to 1.0)
   * * `hue_path` - "shorter" or "longer"
   */
  static interpolate(a: OKLCH, b: OKLCH, t: number, hue_path: string): OKLCH;
  /**
   * Get lightness (0.0 to 1.0).
   */
  readonly l: number;
  /**
   * Get chroma (0.0 to ~0.4).
   */
  readonly c: number;
  /**
   * Get hue (0.0 to 360.0).
   */
  readonly h: number;
}

export class OKLab {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create from Lightness, a (green-red), b (blue-yellow).
   */
  constructor(l: number, a: number, b: number);
  /**
   * Convert from Color.
   */
  static fromColor(color: Color): OKLab;
  /**
   * Convert to Color (sRGB).
   */
  toColor(): Color;
  /**
   * Convert to OKLCH (cylindrical).
   */
  toOklch(): OKLCH;
  /**
   * Linear interpolation in OKLab space.
   */
  static interpolate(from: OKLab, to: OKLab, t: number): OKLab;
  /**
   * Euclidean distance in OKLab.
   */
  deltaE(other: OKLab): number;
  readonly l: number;
  readonly a: number;
  readonly b: number;
}

export class OpticalProperties {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create with custom optical properties
   */
  constructor(absorption_coefficient: number, scattering_coefficient: number, thickness: number, refractive_index: number);
  /**
   * Create default optical properties
   */
  static default(): OpticalProperties;
  /**
   * Get absorption coefficient
   */
  readonly absorptionCoefficient: number;
  /**
   * Get scattering coefficient
   */
  readonly scatteringCoefficient: number;
  /**
   * Get thickness
   */
  readonly thickness: number;
  /**
   * Get refractive index
   */
  readonly refractiveIndex: number;
}

export class PBRMaterial {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static fromPreset(preset: string): PBRMaterial;
  static builder(): PBRMaterialBuilder;
  /**
   * Evaluate the material with default context.
   */
  evaluate(): any;
  /**
   * Evaluate with custom incident angle (cos_theta = angle from normal).
   */
  evaluateAtAngle(cos_theta: number): any;
}

export class PBRMaterialBuilder {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  addDielectric(ior: number, roughness: number): PBRMaterialBuilder;
  addConductor(n: number, k: number, roughness: number): PBRMaterialBuilder;
  addThinFilm(film_ior: number, substrate_ior: number, thickness_nm: number): PBRMaterialBuilder;
  color(r: number, g: number, b: number): PBRMaterialBuilder;
  opacity(opacity: number): PBRMaterialBuilder;
  build(): PBRMaterial;
}

export class PerlinNoise {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new Perlin noise generator
   *
   * # Arguments
   *
   * * `seed` - Random seed for reproducibility
   * * `octaves` - Number of noise layers (1-8)
   * * `persistence` - Amplitude decrease per octave (0.0-1.0)
   * * `lacunarity` - Frequency increase per octave (typically 2.0)
   */
  constructor(seed: number, octaves: number, persistence: number, lacunarity: number);
  /**
   * Generate 2D noise value at position
   *
   * Returns value in range [-1.0, 1.0]
   */
  noise2D(x: number, y: number): number;
  /**
   * Generate fractal (multi-octave) 2D noise
   *
   * Returns value in range [-1.0, 1.0]
   */
  fractalNoise2D(x: number, y: number): number;
  /**
   * Generate RGBA texture buffer
   *
   * # Arguments
   *
   * * `width` - Texture width in pixels
   * * `height` - Texture height in pixels
   * * `scale` - Noise scale factor (typical: 0.01-0.1)
   *
   * # Returns
   *
   * Uint8Array with RGBA values (width * height * 4 bytes)
   */
  generateTexture(width: number, height: number, scale: number): Uint8Array;
  /**
   * Create clear glass noise preset (1 octave)
   */
  static clearGlass(): PerlinNoise;
  /**
   * Create regular glass noise preset (3 octaves)
   */
  static regularGlass(): PerlinNoise;
  /**
   * Create thick glass noise preset (4 octaves)
   */
  static thickGlass(): PerlinNoise;
  /**
   * Create frosted glass noise preset (6 octaves)
   */
  static frostedGlass(): PerlinNoise;
}

/**
 * Polarization state for thin-film calculations
 *
 * # Variants
 * - `S` (TE): Electric field perpendicular to plane of incidence
 * - `P` (TM): Electric field parallel to plane of incidence
 * - `Average`: Unpolarized (average of S and P)
 */
export enum Polarization {
  /**
   * S-polarization (TE, perpendicular)
   */
  S = 0,
  /**
   * P-polarization (TM, parallel)
   */
  P = 1,
  /**
   * Average of S and P (unpolarized)
   */
  Average = 2,
}

export class ProceduralNoise {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a noise generator with explicit parameters.
   *
   * # Arguments
   * * `seed` — deterministic seed (u32)
   * * `octaves` — number of noise layers (1=simple, 6=detailed)
   * * `persistence` — amplitude falloff per octave (0.5 is standard)
   * * `lacunarity` — frequency growth per octave (2.0 is standard)
   */
  constructor(seed: number, octaves: number, persistence: number, lacunarity: number);
  /**
   * Preset: frosted glass (6 octaves — high detail).
   */
  static frosted(): ProceduralNoise;
  /**
   * Preset: regular glass (3 octaves — balanced).
   */
  static regular(): ProceduralNoise;
  /**
   * Preset: clear glass (1 octave — minimal texture).
   */
  static clear(): ProceduralNoise;
  /**
   * Preset: thick glass (4 octaves — more visible texture).
   */
  static thick(): ProceduralNoise;
  /**
   * Sample fractional Brownian motion noise at (x, y).
   *
   * Returns value in `[0, 1]` (normalised — raw Perlin output is [-1, 1]).
   */
  sample(x: number, y: number): number;
  /**
   * Sample the raw Perlin value at (x, y) without normalisation.
   *
   * Returns value in approximately `[-1, 1]`.
   */
  sampleRaw(x: number, y: number): number;
  /**
   * Generate a 2D noise field.
   *
   * Returns a flat array of `cols * rows` values in `[0, 1]`,
   * row-major order (left-to-right, top-to-bottom).
   *
   * # Arguments
   * * `cols` — width in samples
   * * `rows` — height in samples
   * * `scale` — spatial frequency (0.05 = large features, 0.5 = fine detail)
   */
  generateField(cols: number, rows: number, scale: number): Float64Array;
}

export class QualityScore {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Returns whether this score indicates the combination passes requirements.
   */
  passes(): boolean;
  /**
   * Returns a qualitative assessment of the score.
   *
   * Returns: "Excellent", "Good", "Acceptable", "Marginal", or "Poor"
   */
  assessment(): string;
  /**
   * Get confidence level (0.0 to 1.0).
   *
   * Higher confidence means the score is more reliable.
   * For now, returns compliance score as proxy for confidence.
   */
  confidence(): number;
  /**
   * Get human-readable explanation of the score.
   */
  explanation(): string;
  /**
   * Overall quality score (0.0 to 1.0)
   */
  readonly overall: number;
  /**
   * Compliance score (0.0 = fails, 1.0 = exceeds)
   */
  readonly compliance: number;
  /**
   * Perceptual quality score (0.0 = poor, 1.0 = optimal)
   */
  readonly perceptual: number;
  /**
   * Context appropriateness score (0.0 = inappropriate, 1.0 = perfect fit)
   */
  readonly appropriateness: number;
}

export class QualityScorer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new quality scorer.
   */
  constructor();
  /**
   * Score a color combination for a given context.
   *
   * # Arguments
   *
   * * `foreground` - Foreground color
   * * `background` - Background color
   * * `context` - Usage context
   *
   * # Returns
   *
   * Quality score with overall, compliance, perceptual, and appropriateness scores
   */
  score(foreground: Color, background: Color, context: RecommendationContext): QualityScore;
}

export class RateLimiter {
  free(): void;
  [Symbol.dispose](): void;
  constructor(initial: number, max_rate: number, smooth: boolean);
  setTarget(target: number): void;
  update(time: number): number;
  atTarget(): boolean;
  reset(value: number, time: number): void;
  readonly current: number;
  readonly target: number;
}

export class Recommendation {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * The recommended color's RGB components.
   */
  colorRgb(): Uint8Array;
  /**
   * Modification type: "lightness", "chroma", "hue", "combined", "none".
   */
  modificationType(): string;
  /**
   * Get the OKLCH deltas as [deltaL, deltaC, deltaH].
   */
  oklchDeltas(): Float64Array;
  /**
   * The recommended color as hex string.
   */
  readonly hex: string;
  /**
   * Overall quality score (0.0-1.0).
   */
  readonly score: number;
  /**
   * Compliance sub-score.
   */
  readonly compliance: number;
  /**
   * Perceptual quality sub-score.
   */
  readonly perceptual: number;
  /**
   * Appropriateness sub-score.
   */
  readonly appropriateness: number;
  /**
   * Confidence level (0.0-1.0).
   */
  readonly confidence: number;
  /**
   * Human-readable reason for this recommendation.
   */
  readonly reason: string;
  /**
   * Whether the score passes minimum quality threshold.
   */
  readonly passes: boolean;
  /**
   * Assessment grade: "excellent", "good", "acceptable", "poor".
   */
  readonly assessment: string;
}

export class RecommendationContext {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new recommendation context.
   */
  constructor(usage: UsageContext, target: ComplianceTarget);
  /**
   * Create context for body text (WCAG AA).
   */
  static bodyText(): RecommendationContext;
  /**
   * Create context for large text (WCAG AA).
   */
  static largeText(): RecommendationContext;
  /**
   * Create context for interactive elements (WCAG AA).
   */
  static interactive(): RecommendationContext;
  /**
   * Create context for decorative elements (no requirements).
   */
  static decorative(): RecommendationContext;
}

export class RecommendationEngine {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Given a background color, recommend an optimal foreground.
   */
  recommendForeground(bg: Color, usage: number, target: number): Recommendation;
  /**
   * Given an existing (fg, bg) pair, suggest an improved foreground.
   */
  improveForeground(fg: Color, bg: Color, usage: number, target: number): Recommendation;
}

export class RecommendationExplanation {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Full explanation as Markdown.
   */
  toMarkdown(): string;
  /**
   * Number of reasoning points.
   */
  reasoningCount(): number;
  /**
   * Get a reasoning point by index.
   */
  reasoningAt(index: number): any;
  /**
   * Short summary of the recommendation.
   */
  readonly summary: string;
  /**
   * The problem this recommendation addresses.
   */
  readonly problemAddressed: string;
  /**
   * Get benefits as JSON array of strings.
   */
  readonly benefits: any;
  /**
   * Get trade-offs as JSON array of strings.
   */
  readonly tradeOffs: any;
  /**
   * Technical details as JSON.
   */
  readonly technical: any;
}

export class RefractionParams {
  free(): void;
  [Symbol.dispose](): void;
  constructor(index: number, distortion_strength: number, chromatic_aberration: number, edge_lensing: number);
  static clear(): RefractionParams;
  static frosted(): RefractionParams;
  static thick(): RefractionParams;
  static subtle(): RefractionParams;
  static highIndex(): RefractionParams;
  readonly index: number;
  readonly distortionStrength: number;
  readonly chromaticAberration: number;
  readonly edgeLensing: number;
}

export class RenderContext {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create desktop rendering context (1920x1080, sRGB)
   */
  static desktop(): RenderContext;
  /**
   * Create mobile rendering context (375x667, Display P3 if supported)
   */
  static mobile(): RenderContext;
  /**
   * Create 4K rendering context
   */
  static fourK(): RenderContext;
  /**
   * Create custom rendering context
   *
   * # Arguments
   *
   * * `viewport_width` - Viewport width in CSS pixels
   * * `viewport_height` - Viewport height in CSS pixels
   * * `pixel_density` - Device pixel density (1.0 = standard, 2.0 = retina)
   */
  constructor(viewport_width: number, viewport_height: number, pixel_density: number);
  /**
   * Get viewport width
   */
  readonly viewportWidth: number;
  /**
   * Get viewport height
   */
  readonly viewportHeight: number;
  /**
   * Get pixel density
   */
  readonly pixelDensity: number;
}

export class SellmeierDispersion {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create custom Sellmeier dispersion model.
   *
   * # Arguments
   *
   * * `b` - Array of 3 oscillator strengths [B1, B2, B3]
   * * `c` - Array of 3 resonance wavelengths squared in μm² [C1, C2, C3]
   */
  constructor(b: Float64Array, c: Float64Array);
  /**
   * Fused silica (SiO2) - Malitson 1965.
   */
  static fusedSilica(): SellmeierDispersion;
  /**
   * BK7 optical glass (Schott) - Common crown glass.
   */
  static bk7(): SellmeierDispersion;
  /**
   * SF11 flint glass (Schott) - High dispersion.
   */
  static sf11(): SellmeierDispersion;
  /**
   * Sapphire (Al2O3) - Ordinary ray.
   */
  static sapphire(): SellmeierDispersion;
  /**
   * Diamond (C).
   */
  static diamond(): SellmeierDispersion;
  /**
   * Calculate refractive index at given wavelength.
   *
   * More accurate than Cauchy, especially in UV and IR ranges.
   */
  n(wavelength_nm: number): number;
  /**
   * Calculate refractive indices for RGB channels.
   */
  nRgb(): Float64Array;
  /**
   * Calculate Abbe number.
   */
  abbeNumber(): number;
  /**
   * Get base refractive index (at sodium d-line).
   */
  nBase(): number;
}

export class SirenCorrection {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  readonly deltaL: number;
  readonly deltaC: number;
  readonly deltaH: number;
}

export class SpectralComplexIOR {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create spectral complex IOR from RGB values.
   *
   * # Arguments
   *
   * * `n_rgb` - Array of n values [n_red, n_green, n_blue]
   * * `k_rgb` - Array of k values [k_red, k_green, k_blue]
   */
  constructor(n_rgb: Float64Array, k_rgb: Float64Array);
  /**
   * Gold (Au) - Warm yellow metal.
   * Source: Johnson & Christy (1972)
   */
  static gold(): SpectralComplexIOR;
  /**
   * Silver (Ag) - Neutral white metal, highest reflectivity.
   * Source: Johnson & Christy (1972)
   */
  static silver(): SpectralComplexIOR;
  /**
   * Copper (Cu) - Orange-red metal.
   * Source: Johnson & Christy (1972)
   */
  static copper(): SpectralComplexIOR;
  /**
   * Aluminum (Al) - Bright white metal, slight blue tint.
   * Source: Rakic (1995)
   */
  static aluminum(): SpectralComplexIOR;
  /**
   * Iron (Fe) - Dark gray metal.
   * Source: Johnson & Christy (1974)
   */
  static iron(): SpectralComplexIOR;
  /**
   * Chromium (Cr) - Bright silver metal.
   */
  static chromium(): SpectralComplexIOR;
  /**
   * Titanium (Ti) - Dark silver with yellow tint.
   */
  static titanium(): SpectralComplexIOR;
  /**
   * Nickel (Ni) - Warm silver metal.
   */
  static nickel(): SpectralComplexIOR;
  /**
   * Platinum (Pt) - Dense silver-white metal.
   */
  static platinum(): SpectralComplexIOR;
  /**
   * Brass (Cu-Zn alloy) - Yellow metal.
   */
  static brass(): SpectralComplexIOR;
  /**
   * Bronze (Cu-Sn alloy) - Brown metal.
   */
  static bronze(): SpectralComplexIOR;
  /**
   * Tungsten (W) - Dense gray metal.
   */
  static tungsten(): SpectralComplexIOR;
  /**
   * Get F0 (normal incidence reflectance) for each RGB channel.
   *
   * THIS IS THE METAL'S COLOR - it emerges from the spectral response!
   *
   * # Returns
   *
   * Array [F0_red, F0_green, F0_blue]
   */
  f0Rgb(): Float64Array;
  /**
   * Calculate full Fresnel reflectance for each RGB channel.
   *
   * Uses exact complex Fresnel equations for conductors.
   *
   * # Arguments
   *
   * * `n_i` - Incident medium IOR (1.0 for air)
   * * `cos_theta_i` - Cosine of incident angle
   *
   * # Returns
   *
   * Array [R_red, R_green, R_blue]
   */
  fresnelRgb(n_i: number, cos_theta_i: number): Float64Array;
  /**
   * Calculate Schlick approximation for each RGB channel.
   *
   * Faster than full Fresnel, ~10% error.
   */
  fresnelSchlickRgb(cos_theta_i: number): Float64Array;
  /**
   * Generate CSS for metallic gradient effect.
   */
  toCssGradient(intensity: number): string;
  /**
   * Generate CSS for metallic surface with light angle.
   */
  toCssSurface(light_angle_deg: number): string;
  /**
   * Get IOR at red wavelength (~650nm).
   */
  readonly red: ComplexIOR;
  /**
   * Get IOR at green wavelength (~550nm).
   */
  readonly green: ComplexIOR;
  /**
   * Get IOR at blue wavelength (~450nm).
   */
  readonly blue: ComplexIOR;
}

export class SpectralPipeline {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create empty pipeline
   */
  constructor();
  /**
   * Add thin film interference stage
   *
   * # Arguments
   * * `n_film` - Film refractive index
   * * `thickness_nm` - Film thickness in nanometers
   * * `n_substrate` - Substrate refractive index
   */
  addThinFilm(n_film: number, thickness_nm: number, n_substrate: number): void;
  /**
   * Add dispersion stage (Cauchy model)
   *
   * # Arguments
   * * `a` - Cauchy A coefficient
   * * `b` - Cauchy B coefficient
   * * `c` - Cauchy C coefficient
   */
  addDispersion(a: number, b: number, c: number): void;
  /**
   * Add crown glass dispersion
   */
  addCrownGlassDispersion(): void;
  /**
   * Add Mie scattering stage
   *
   * # Arguments
   * * `radius_um` - Particle radius in micrometers
   * * `n_particle` - Particle refractive index
   * * `n_medium` - Medium refractive index
   */
  addMieScattering(radius_um: number, n_particle: number, n_medium: number): void;
  /**
   * Add fog scattering
   */
  addFog(): void;
  /**
   * Add thermo-optic stage
   *
   * # Arguments
   * * `n_base` - Base refractive index at reference temperature
   * * `dn_dt` - Thermo-optic coefficient (dn/dT) in K⁻¹
   * * `thickness_nm` - Film thickness in nm
   * * `alpha_thermal` - Thermal expansion coefficient in K⁻¹
   */
  addThermoOptic(n_base: number, dn_dt: number, thickness_nm: number, alpha_thermal: number): void;
  /**
   * Add gold metal reflectance
   */
  addGold(): void;
  /**
   * Add silver metal reflectance
   */
  addSilver(): void;
  /**
   * Add copper metal reflectance
   */
  addCopper(): void;
  /**
   * Evaluate the complete pipeline
   *
   * # Arguments
   * * `incident` - Incident light spectrum
   * * `context` - Evaluation context
   *
   * # Returns
   * Final spectral signal (use `.toRgb()` to convert to color)
   */
  evaluate(incident: SpectralSignal, context: EvaluationContext): SpectralSignal;
  /**
   * Evaluate and return intermediate results for visualization
   *
   * # Returns
   * Array of { name: string, wavelengths: number[], intensities: number[] }
   */
  evaluateWithIntermediates(incident: SpectralSignal, context: EvaluationContext): Array<any>;
  /**
   * Get stage names
   */
  stageNames(): Array<any>;
  /**
   * Verify energy conservation
   */
  verifyEnergyConservation(incident: SpectralSignal, context: EvaluationContext): boolean;
  /**
   * Get number of stages
   */
  readonly stageCount: number;
}

export class SpectralSignal {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create from wavelengths and intensities arrays
   *
   * # Arguments
   * * `wavelengths` - Wavelengths in nm
   * * `intensities` - Intensity values (0.0 to 1.0 for reflectance)
   */
  constructor(wavelengths: Float64Array, intensities: Float64Array);
  /**
   * Create uniform (flat) spectrum at given intensity
   *
   * # Arguments
   * * `intensity` - Uniform intensity value
   */
  static uniformDefault(intensity: number): SpectralSignal;
  /**
   * Create D65 daylight illuminant (normalized white light)
   */
  static d65Illuminant(): SpectralSignal;
  /**
   * Get interpolated intensity at arbitrary wavelength
   *
   * # Arguments
   * * `wavelength_nm` - Wavelength in nanometers
   */
  intensityAt(wavelength_nm: number): number;
  /**
   * Get all wavelengths
   */
  wavelengths(): Float64Array;
  /**
   * Get all intensities
   */
  intensities(): Float64Array;
  /**
   * Get total integrated energy
   */
  totalEnergy(): number;
  /**
   * Convert to CIE XYZ color space
   *
   * # Returns
   * [X, Y, Z] tristimulus values
   */
  toXyz(): Float64Array;
  /**
   * Convert to sRGB
   *
   * # Returns
   * [R, G, B] in 0.0-1.0 range
   *
   * **THIS IS THE ONLY PLACE WHERE RGB IS COMPUTED.**
   * All physics happens in spectral domain before this.
   */
  toRgb(): Float64Array;
  /**
   * Convert to sRGB as u8 values
   *
   * # Returns
   * [R, G, B] in 0-255 range
   */
  toRgbU8(): Uint8Array;
  /**
   * Multiply by another signal (element-wise, with interpolation)
   */
  multiply(other: SpectralSignal): SpectralSignal;
  /**
   * Scale by constant factor
   */
  scale(factor: number): SpectralSignal;
  /**
   * Get number of samples
   */
  readonly sampleCount: number;
}

export class StepSelector {
  free(): void;
  [Symbol.dispose](): void;
  constructor(goal_type: string, target: number);
  /**
   * Update current progress value.
   */
  updateProgress(value: number): void;
  /**
   * Record outcome of a step execution.
   */
  recordOutcome(step_type: string, improvement: number, cost: number, success: boolean): void;
  /**
   * Get the next recommended step as JSON, or null if goal achieved.
   */
  recommendNextStep(): any;
  /**
   * Current goal progress (0.0-1.0).
   */
  goalProgress(): number;
  /**
   * Whether the goal has been achieved.
   */
  isGoalAchieved(): boolean;
}

export enum TargetMediumEnum {
  Screen = 0,
  Print = 1,
  Projection = 2,
}

export class TempOxidizedMetal {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Set temperature
   *
   * # Arguments
   * * `temp_k` - Temperature in Kelvin
   *
   * Valid range: 200K to 1500K (outside may be unphysical)
   */
  setTemperature(temp_k: number): void;
  /**
   * Set oxidation level
   *
   * # Arguments
   * * `level` - Oxidation level (0.0 = fresh, 1.0 = heavily oxidized)
   *
   * 0.0 = bare metal, native oxide only
   * 0.3 = light tarnish
   * 0.7 = significant oxidation
   * 1.0 = maximum oxidation (patina/rust)
   */
  setOxidation(level: number): void;
  /**
   * Get metal spectral IOR at current temperature
   *
   * Returns [n_R, k_R, n_G, k_G, n_B, k_B]
   */
  metalSpectralIor(): Float64Array;
  /**
   * Get effective reflectance at wavelength including oxide layer
   *
   * # Arguments
   * * `wavelength_nm` - Wavelength in nanometers
   * * `cos_theta` - Cosine of incidence angle (1.0 = normal)
   */
  effectiveReflectance(wavelength_nm: number, cos_theta: number): number;
  /**
   * Get effective RGB reflectance
   *
   * # Arguments
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   * [R, G, B] reflectance (0.0 to 1.0)
   */
  effectiveReflectanceRgb(cos_theta: number): Float64Array;
  /**
   * Generate CSS for temperature-dependent metal effect
   *
   * # Arguments
   * * `light_angle_deg` - Light angle in degrees
   */
  toCssTempMetal(light_angle_deg: number): string;
  /**
   * Generate CSS for patina effect
   */
  toCssPatina(): string;
  /**
   * Get F0 (normal incidence reflectance) at RGB wavelengths
   *
   * This is the key method for emergent color - color comes from
   * the spectral F0 response, not hardcoded values.
   *
   * # Returns
   * [R, G, B] reflectance at normal incidence (0.0 to 1.0)
   */
  f0Rgb(): Float64Array;
  /**
   * Get effective oxide layer thickness in nanometers
   *
   * The oxide thickness varies with oxidation level:
   * - 0.0 = native oxide only (~2-5nm)
   * - 0.5 = moderate oxidation (~50nm)
   * - 1.0 = heavy oxidation/patina (~200nm+)
   */
  effectiveOxideThickness(): number;
  /**
   * Generate CSS gradient from temperature-dependent physics
   *
   * Convenience method that combines temperature and oxidation effects
   * into a single CSS gradient suitable for UI display.
   */
  toCssGradient(): string;
  /**
   * Fresh copper (no oxidation, room temperature)
   */
  static copperFresh(): TempOxidizedMetal;
  /**
   * Tarnished copper (light oxidation)
   */
  static copperTarnished(): TempOxidizedMetal;
  /**
   * Copper with patina (heavy oxidation)
   */
  static copperPatina(): TempOxidizedMetal;
  /**
   * Fresh silver
   */
  static silverFresh(): TempOxidizedMetal;
  /**
   * Tarnished silver
   */
  static silverTarnished(): TempOxidizedMetal;
  /**
   * Fresh aluminum (with native oxide)
   */
  static aluminumFresh(): TempOxidizedMetal;
  /**
   * Rusty iron
   */
  static ironRusty(): TempOxidizedMetal;
  /**
   * Hot gold (elevated temperature)
   */
  static goldHot(): TempOxidizedMetal;
  readonly temperatureK: number;
  readonly oxidationLevel: number;
}

export class TemporalConductor {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create with heated gold preset.
   */
  static heatedGold(): TemporalConductor;
}

export class TemporalConductorMaterial {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a temporal conductor with custom parameters.
   *
   * # Arguments
   * * `n_base` — real part of IOR at reference temperature
   * * `k_base` — extinction coefficient at reference temperature
   * * `roughness` — surface roughness (0=mirror, 1=rough)
   * * `n_temp_coeff` — dn/dT (temperature coefficient for n)
   * * `k_temp_coeff` — dk/dT (temperature coefficient for k)
   */
  constructor(n_base: number, k_base: number, roughness: number, n_temp_coeff: number, k_temp_coeff: number);
  /**
   * Preset: heated gold (reddish at high T).
   */
  static heatedGold(): TemporalConductorMaterial;
  /**
   * Preset: heated copper.
   */
  static heatedCopper(): TemporalConductorMaterial;
  /**
   * Evaluate BSDF at given time and temperature.
   *
   * # Arguments
   * * `temperature_k` — temperature in Kelvin (293.15 = 20°C)
   * * `cos_theta` — cosine of incident angle
   *
   * Returns `[reflectance, transmittance, absorption]`.
   */
  evalAtTemperature(temperature_k: number, cos_theta: number): Float64Array;
  /**
   * Evaluate at room temperature (293.15 K = 20°C).
   */
  evalAtRoomTemp(cos_theta: number): Float64Array;
}

export class TemporalDielectric {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create with drying paint preset.
   */
  static dryingPaint(): TemporalDielectric;
  /**
   * Create with weathering glass preset.
   */
  static weatheringGlass(): TemporalDielectric;
}

export class TemporalMaterial {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a temporal dielectric with custom roughness evolution.
   *
   * # Arguments
   * * `roughness_base` — initial roughness at t=0 (0=mirror, 1=fully rough)
   * * `roughness_target` — final roughness as t→∞
   * * `roughness_tau` — time constant in seconds (e.g. 60 = dries in ~1 min)
   * * `ior_base` — index of refraction (e.g. 1.52 for glass)
   */
  constructor(roughness_base: number, roughness_target: number, roughness_tau: number, ior_base: number);
  /**
   * Preset: drying paint (roughness 0.05→0.4 over ~60s).
   */
  static dryingPaint(): TemporalMaterial;
  /**
   * Preset: weathering glass (roughness 0.01→0.15 over ~1h).
   */
  static weatheringGlass(): TemporalMaterial;
  /**
   * Evaluate BSDF at given time and angle.
   *
   * Returns `[reflectance, transmittance, absorption]` — always sums to 1.0.
   *
   * # Arguments
   * * `t` — simulation time in seconds
   * * `cos_theta` — cosine of incident angle (0=grazing, 1=normal)
   */
  evalAtTime(t: number, cos_theta: number): Float64Array;
  /**
   * Evaluate at t=0 (static fallback — backward-compatible).
   */
  evalStatic(cos_theta: number): Float64Array;
  /**
   * Whether this material has time-varying behaviour.
   */
  readonly supportsTemoral: boolean;
}

export class TemporalThinFilm {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create with soap bubble preset.
   */
  static soapBubble(): TemporalThinFilm;
}

export class TemporalThinFilmMaterial {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a temporal thin-film with custom parameters.
   *
   * # Arguments
   * * `thickness_base` — base film thickness in nm (e.g. 300)
   * * `amplitude` — oscillation amplitude in nm
   * * `frequency_hz` — oscillation frequency in Hz
   * * `film_ior` — film IOR (e.g. 1.33 for water)
   * * `substrate_ior` — substrate IOR (e.g. 1.0 for air)
   */
  constructor(thickness_base: number, amplitude: number, frequency_hz: number, film_ior: number, substrate_ior: number);
  /**
   * Preset: soap bubble (300nm base, 100nm amplitude, 2 Hz, damped).
   */
  static soapBubble(): TemporalThinFilmMaterial;
  /**
   * Preset: oil slick (400nm base, slow oscillation).
   */
  static oilSlick(): TemporalThinFilmMaterial;
  /**
   * Evaluate BSDF at given time and angle.
   *
   * Returns `[reflectance, transmittance, absorption]` — always sums to 1.0.
   */
  evalAtTime(t: number, cos_theta: number): Float64Array;
  /**
   * Sample reflectance values across a time range.
   *
   * Returns flat `[t0, r0, t1, r1, ...]` for `samples` points in `[0, duration]`.
   */
  sampleTimeline(duration: number, samples: number, cos_theta: number): Float64Array;
}

export class ThinFilm {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new thin film with custom parameters.
   *
   * # Arguments
   *
   * * `n_film` - Film refractive index (typically 1.3-1.7)
   * * `thickness_nm` - Film thickness in nanometers (typically 50-500nm)
   *
   * # Example
   *
   * ```javascript
   * // Custom thin film: n=1.45, thickness=180nm
   * const film = new ThinFilm(1.45, 180.0);
   * ```
   */
  constructor(n_film: number, thickness_nm: number);
  /**
   * Thin soap bubble (~100nm water film).
   *
   * Creates subtle blue-violet interference colors.
   */
  static soapBubbleThin(): ThinFilm;
  /**
   * Medium soap bubble (~200nm water film).
   *
   * Creates balanced rainbow interference colors.
   */
  static soapBubbleMedium(): ThinFilm;
  /**
   * Thick soap bubble (~400nm water film).
   *
   * Creates stronger yellow-red interference colors.
   */
  static soapBubbleThick(): ThinFilm;
  /**
   * Thin oil slick on water (~150nm).
   *
   * Oil (n≈1.5) on water (n≈1.33) creates classic rainbow effect.
   */
  static oilThin(): ThinFilm;
  /**
   * Medium oil slick (~300nm).
   */
  static oilMedium(): ThinFilm;
  /**
   * Thick oil slick (~500nm).
   */
  static oilThick(): ThinFilm;
  /**
   * Anti-reflective coating (MgF2 on glass).
   *
   * Quarter-wave thickness at 550nm for minimal reflection.
   */
  static arCoating(): ThinFilm;
  /**
   * Thin oxide layer (SiO2 on silicon, ~50nm).
   *
   * Creates characteristic chip colors.
   */
  static oxideThin(): ThinFilm;
  /**
   * Medium oxide layer (~150nm).
   */
  static oxideMedium(): ThinFilm;
  /**
   * Thick oxide layer (~300nm).
   */
  static oxideThick(): ThinFilm;
  /**
   * Beetle shell coating (chitin-like material).
   *
   * Creates natural iridescence seen in jewel beetles.
   */
  static beetleShell(): ThinFilm;
  /**
   * Pearl nacre (aragonite layers).
   *
   * Creates lustrous pearl iridescence.
   */
  static nacre(): ThinFilm;
  /**
   * Calculate optical path difference for given viewing angle.
   *
   * OPD = 2 * n_film * d * cos(theta_film)
   *
   * # Arguments
   *
   * * `cos_theta_air` - Cosine of incidence angle in air (1.0 = normal)
   *
   * # Returns
   *
   * Optical path difference in nanometers
   */
  opticalPathDifference(cos_theta_air: number): number;
  /**
   * Calculate phase difference for a given wavelength.
   *
   * delta = 2 * PI * OPD / lambda
   *
   * # Arguments
   *
   * * `wavelength_nm` - Wavelength in nanometers (visible: 400-700nm)
   * * `cos_theta` - Cosine of incidence angle (1.0 = normal)
   *
   * # Returns
   *
   * Phase difference in radians
   */
  phaseDifference(wavelength_nm: number, cos_theta: number): number;
  /**
   * Calculate reflectance at a single wavelength using the Airy formula.
   *
   * This is the core physics calculation that accounts for:
   * - Fresnel reflection at both interfaces
   * - Phase difference from optical path
   * - Interference between reflected rays
   *
   * # Arguments
   *
   * * `wavelength_nm` - Wavelength in nanometers (visible: 400-700nm)
   * * `n_substrate` - Substrate refractive index (air=1.0, water=1.33, glass=1.52)
   * * `cos_theta` - Cosine of incidence angle (1.0 = normal, 0.0 = grazing)
   *
   * # Returns
   *
   * Reflectance (0.0-1.0)
   *
   * # Example
   *
   * ```javascript
   * const film = ThinFilm.soapBubbleMedium();
   *
   * // Green light at normal incidence, air substrate
   * const rGreen = film.reflectance(550.0, 1.0, 1.0);
   *
   * // Same but at 60° angle
   * const rAngled = film.reflectance(550.0, 1.0, 0.5);  // cos(60°) = 0.5
   * ```
   */
  reflectance(wavelength_nm: number, n_substrate: number, cos_theta: number): number;
  /**
   * Calculate RGB reflectance (R=650nm, G=550nm, B=450nm).
   *
   * Returns reflectance values for rendering colored interference.
   *
   * # Arguments
   *
   * * `n_substrate` - Substrate refractive index
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   *
   * Array of 3 reflectance values [R, G, B] in range 0.0-1.0
   *
   * # Example
   *
   * ```javascript
   * const film = ThinFilm.oilMedium();
   * const rgb = film.reflectanceRgb(1.33, 0.8);  // oil on water
   * console.log(`R=${rgb[0]}, G=${rgb[1]}, B=${rgb[2]}`);
   * ```
   */
  reflectanceRgb(n_substrate: number, cos_theta: number): Float64Array;
  /**
   * Calculate full spectrum reflectance (8 wavelengths: 400-750nm).
   *
   * Returns wavelengths and corresponding reflectances for spectral rendering.
   *
   * # Arguments
   *
   * * `n_substrate` - Substrate refractive index
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   *
   * Object with `wavelengths` (8 values) and `reflectances` (8 values)
   */
  reflectanceSpectrum(n_substrate: number, cos_theta: number): object;
  /**
   * Find wavelength of maximum constructive interference.
   *
   * For first-order maximum: OPD = lambda
   *
   * # Arguments
   *
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   *
   * Wavelength in nanometers where reflectance is maximized
   */
  maxWavelength(cos_theta: number): number;
  /**
   * Find wavelength of maximum destructive interference.
   *
   * For first-order minimum: OPD = lambda/2
   *
   * # Arguments
   *
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   *
   * Wavelength in nanometers where reflectance is minimized
   */
  minWavelength(cos_theta: number): number;
  /**
   * Generate CSS for soap bubble effect.
   *
   * Creates a radial gradient that simulates angle-dependent
   * interference colors with a highlight at the center.
   *
   * # Arguments
   *
   * * `size_percent` - Size scaling percentage (100 = full size)
   *
   * # Returns
   *
   * CSS radial-gradient string
   *
   * # Example
   *
   * ```javascript
   * const film = ThinFilm.soapBubbleMedium();
   * const css = film.toCssSoapBubble(100.0);
   * element.style.background = css;
   * ```
   */
  toCssSoapBubble(size_percent: number): string;
  /**
   * Generate CSS for oil slick effect.
   *
   * Creates a linear gradient that simulates rainbow-like
   * interference patterns seen on oil films.
   *
   * # Returns
   *
   * CSS linear-gradient string
   *
   * # Example
   *
   * ```javascript
   * const film = ThinFilm.oilMedium();
   * const css = film.toCssOilSlick();
   * element.style.background = css;
   * ```
   */
  toCssOilSlick(): string;
  /**
   * Generate CSS for general iridescent gradient.
   *
   * Creates a gradient with angle-dependent color shift over a base color.
   *
   * # Arguments
   *
   * * `n_substrate` - Substrate refractive index
   * * `base_color` - Base CSS color string (e.g., "#000000")
   *
   * # Returns
   *
   * CSS gradient string
   */
  toCssIridescentGradient(n_substrate: number, base_color: string): string;
  /**
   * Convert thin-film reflectance to RGB color for given conditions.
   *
   * # Arguments
   *
   * * `n_substrate` - Substrate refractive index
   * * `cos_theta` - Cosine of incidence angle
   *
   * # Returns
   *
   * Array [r, g, b] with values 0-255
   */
  toRgb(n_substrate: number, cos_theta: number): Uint8Array;
  /**
   * Film refractive index.
   */
  readonly nFilm: number;
  /**
   * Film thickness in nanometers.
   */
  readonly thicknessNm: number;
}

export class ThinFilmBSDF {
  free(): void;
  [Symbol.dispose](): void;
  constructor(substrate_ior: number, film_ior: number, film_thickness: number);
  static soapBubble(thickness: number): ThinFilmBSDF;
  static oilOnWater(thickness: number): ThinFilmBSDF;
  static arCoating(): ThinFilmBSDF;
  evaluate(wi_x: number, wi_y: number, wi_z: number, wo_x: number, wo_y: number, wo_z: number): any;
}

export class TransferMatrixFilm {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new empty film stack
   *
   * # Arguments
   * * `n_incident` - Incident medium refractive index (typically 1.0 for air)
   * * `n_substrate` - Substrate refractive index (e.g., 1.52 for glass)
   */
  constructor(n_incident: number, n_substrate: number);
  /**
   * Add a dielectric layer to the stack
   *
   * Layers are added from the incident medium side toward the substrate.
   *
   * # Arguments
   * * `n` - Real refractive index of the layer
   * * `thickness_nm` - Layer thickness in nanometers
   */
  addLayer(n: number, thickness_nm: number): void;
  /**
   * Add an absorbing layer with complex IOR
   *
   * # Arguments
   * * `n` - Real part of refractive index
   * * `k` - Extinction coefficient
   * * `thickness_nm` - Layer thickness in nanometers
   */
  addAbsorbingLayer(n: number, k: number, thickness_nm: number): void;
  /**
   * Create a Bragg mirror (distributed Bragg reflector)
   *
   * Alternating high/low index quarter-wave layers create
   * wavelength-selective high reflectance.
   *
   * # Arguments
   * * `n_high` - High index material (e.g., TiO2 = 2.35)
   * * `n_low` - Low index material (e.g., SiO2 = 1.46)
   * * `design_lambda` - Design wavelength in nm
   * * `pairs` - Number of layer pairs
   *
   * # Physics
   *
   * Stop band width ∝ (n_high - n_low) / (n_high + n_low)
   * Peak reflectance increases exponentially with pairs.
   *
   * # Example
   * ```javascript
   * // Green-reflecting Bragg mirror
   * const mirror = TransferMatrixFilm.braggMirror(2.35, 1.46, 550.0, 10);
   * console.log(`R @ 550nm: ${mirror.reflectance(550.0, 0.0, Polarization.Average)}`);
   * ```
   */
  static braggMirror(n_high: number, n_low: number, design_lambda: number, pairs: number): TransferMatrixFilm;
  /**
   * Create a broadband anti-reflection coating
   *
   * Two-layer V-coat design for glass substrates.
   *
   * # Arguments
   * * `design_lambda` - Center wavelength in nm (typically 550)
   */
  static arBroadband(design_lambda: number): TransferMatrixFilm;
  /**
   * Create a notch filter (narrow rejection band)
   *
   * # Arguments
   * * `center_lambda` - Center wavelength to reject in nm
   * * `bandwidth_nm` - Approximate bandwidth in nm
   */
  static notchFilter(center_lambda: number, bandwidth_nm: number): TransferMatrixFilm;
  /**
   * Create a dichroic filter that reflects blue, transmits red/green
   *
   * Used in color separation and stage lighting.
   */
  static dichroicBlueReflect(): TransferMatrixFilm;
  /**
   * Create a dichroic filter that reflects red, transmits blue/green
   */
  static dichroicRedReflect(): TransferMatrixFilm;
  /**
   * Create a Morpho butterfly wing structure
   *
   * # Physics
   *
   * The brilliant blue of Morpho butterflies is NOT from pigment.
   * It emerges from irregularly-spaced chitin/air layers that
   * create broadband constructive interference for blue light.
   *
   * Key characteristics:
   * - Chitin (n ≈ 1.56) and air (n = 1.0) layers
   * - Irregular spacing creates broadband reflection
   * - Strong angle dependence (iridescence)
   *
   * # Example
   * ```javascript
   * const morpho = TransferMatrixFilm.morphoButterfly();
   *
   * // Blue emerges from STRUCTURE
   * const rgb = morpho.reflectanceRgb(0.0, Polarization.Average);
   * // rgb[2] (blue) >> rgb[0] (red)
   *
   * // Color shifts with angle
   * const rgb45 = morpho.reflectanceRgb(45.0, Polarization.Average);
   * // Blue shifts toward UV at oblique angles
   * ```
   */
  static morphoButterfly(): TransferMatrixFilm;
  /**
   * Create a beetle shell iridescence structure
   *
   * Gradual index variation creates metallic-looking iridescence.
   */
  static beetleShell(): TransferMatrixFilm;
  /**
   * Create a nacre (mother of pearl) structure
   *
   * # Physics
   *
   * Nacre is made of aragonite (CaCO3) platelets in a protein matrix.
   * The alternating high/low index creates pearlescent iridescence.
   *
   * - Aragonite: n ≈ 1.68
   * - Protein matrix: n ≈ 1.34
   * - ~20 platelet layers
   */
  static nacre(): TransferMatrixFilm;
  /**
   * Create an optical disc (CD/DVD) approximation
   *
   * Polycarbonate with thin metallic reflection layer.
   */
  static opticalDisc(): TransferMatrixFilm;
  /**
   * Calculate reflectance at a single wavelength and angle
   *
   * # Arguments
   * * `wavelength_nm` - Wavelength in nanometers
   * * `angle_deg` - Incidence angle in degrees (0 = normal)
   * * `pol` - Polarization state (S, P, or Average)
   *
   * # Returns
   * Reflectance (0.0 to 1.0)
   *
   * # Example
   * ```javascript
   * const mirror = TransferMatrixFilm.braggMirror(2.35, 1.46, 550.0, 10);
   *
   * // At design wavelength
   * const r = mirror.reflectance(550.0, 0.0, Polarization.Average);
   * // r > 0.95 (high reflectance)
   *
   * // Off-band
   * const r_off = mirror.reflectance(700.0, 0.0, Polarization.Average);
   * // r_off << r (low reflectance outside stop band)
   * ```
   */
  reflectance(wavelength_nm: number, angle_deg: number, pol: Polarization): number;
  /**
   * Calculate transmittance at a single wavelength and angle
   *
   * # Arguments
   * * `wavelength_nm` - Wavelength in nanometers
   * * `angle_deg` - Incidence angle in degrees
   * * `pol` - Polarization state
   *
   * # Returns
   * Transmittance (0.0 to 1.0)
   *
   * # Note
   * For lossless films: R + T ≈ 1
   */
  transmittance(wavelength_nm: number, angle_deg: number, pol: Polarization): number;
  /**
   * Calculate RGB reflectance (R=650nm, G=550nm, B=450nm)
   *
   * # Arguments
   * * `angle_deg` - Incidence angle in degrees
   * * `pol` - Polarization state
   *
   * # Returns
   * Array [R, G, B] reflectance values (0.0 to 1.0)
   *
   * # Example
   * ```javascript
   * const morpho = TransferMatrixFilm.morphoButterfly();
   *
   * // Color EMERGES from structure
   * const rgb = morpho.reflectanceRgb(0.0, Polarization.Average);
   *
   * // For Morpho: B >> R (structural blue)
   * console.log(`R=${rgb[0].toFixed(2)}, G=${rgb[1].toFixed(2)}, B=${rgb[2].toFixed(2)}`);
   * ```
   */
  reflectanceRgb(angle_deg: number, pol: Polarization): Float64Array;
  /**
   * Calculate full spectrum reflectance
   *
   * # Arguments
   * * `angle_deg` - Incidence angle in degrees
   * * `pol` - Polarization state
   * * `num_points` - Number of spectral points
   *
   * # Returns
   * Object { wavelengths: number[], reflectances: number[] }
   */
  reflectanceSpectrum(angle_deg: number, pol: Polarization, num_points: number): object;
  /**
   * Generate CSS gradient for structural color effect
   *
   * Samples reflectance at multiple angles to create an
   * iridescent gradient showing the angle-dependent color.
   *
   * # Returns
   * CSS linear-gradient string
   *
   * # Example
   * ```javascript
   * const morpho = TransferMatrixFilm.morphoButterfly();
   * element.style.background = morpho.toCssStructuralColor();
   * ```
   */
  toCssStructuralColor(): string;
  /**
   * Get the number of layers in the stack
   */
  readonly layerCount: number;
  /**
   * Get incident medium refractive index
   */
  readonly nIncident: number;
  /**
   * Get substrate refractive index
   */
  readonly nSubstrate: number;
}

/**
 * Usage context for color recommendations.
 */
export enum UsageContext {
  /**
   * Body text - primary content (18px or less, normal weight)
   */
  BodyText = 0,
  /**
   * Large text - headings, titles (18pt+ or 14pt+ bold)
   */
  LargeText = 1,
  /**
   * Interactive elements - buttons, links, form inputs
   */
  Interactive = 2,
  /**
   * Decorative - non-essential visual elements
   */
  Decorative = 3,
  /**
   * Icons and graphics - functional imagery
   */
  IconsGraphics = 4,
  /**
   * Disabled state - reduced emphasis
   */
  Disabled = 5,
}

export class Vec3 {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new 3D vector
   */
  constructor(x: number, y: number, z: number);
  /**
   * Normalize vector to unit length
   */
  normalize(): Vec3;
  /**
   * Calculate dot product with another vector
   */
  dot(other: Vec3): number;
  /**
   * Reflect vector around normal
   */
  reflect(normal: Vec3): Vec3;
  /**
   * Get x component
   */
  readonly x: number;
  /**
   * Get y component
   */
  readonly y: number;
  /**
   * Get z component
   */
  readonly z: number;
}

export class VibrancyEffect {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create new vibrancy effect.
   */
  constructor(level: VibrancyLevel);
  /**
   * Apply vibrancy to foreground color given background.
   */
  apply(foreground: OKLCH, background: OKLCH): OKLCH;
  /**
   * Get vibrancy level.
   */
  readonly level: VibrancyLevel;
}

/**
 * Vibrancy level determines how much background color bleeds through.
 */
export enum VibrancyLevel {
  /**
   * Primary vibrancy - most color through (75%)
   */
  Primary = 0,
  /**
   * Secondary vibrancy - moderate color (50%)
   */
  Secondary = 1,
  /**
   * Tertiary vibrancy - subtle color (30%)
   */
  Tertiary = 2,
  /**
   * Divider vibrancy - minimal color (15%)
   */
  Divider = 3,
}

export class ViewContext {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Perpendicular view preset
   */
  static perpendicular(): ViewContext;
  /**
   * Oblique view preset (45° angle)
   */
  static oblique(): ViewContext;
  /**
   * Grazing angle view preset
   */
  static grazing(): ViewContext;
}

export class WCAGMetric {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Create a new WCAG metric.
   */
  constructor();
  /**
   * Evaluate contrast between foreground and background colors.
   *
   * Returns a contrast ratio from 1.0 to 21.0.
   */
  evaluate(foreground: Color, background: Color): ContrastResult;
  /**
   * Evaluate contrast for multiple color pairs (faster than calling evaluate in a loop).
   *
   * # Arguments
   *
   * * `foregrounds` - Array of foreground colors
   * * `backgrounds` - Array of background colors (must match length)
   *
   * # Returns
   *
   * Array of contrast results
   */
  evaluateBatch(foregrounds: Color[], backgrounds: Color[]): ContrastResult[];
  /**
   * Check if contrast ratio passes WCAG level for text size.
   *
   * # Arguments
   *
   * * `ratio` - Contrast ratio to check
   * * `level` - "AA" or "AAA"
   * * `is_large_text` - Whether text is large (18pt+ or 14pt+ bold)
   */
  static passes(ratio: number, level: string, is_large_text: boolean): boolean;
}

/**
 * Harmony type selector for WASM.
 */
export enum WasmHarmonyType {
  Complementary = 0,
  SplitComplementary = 1,
  Triadic = 2,
  Tetradic = 3,
  Analogous = 4,
  Monochromatic = 5,
  TemperatureWarm = 6,
  TemperatureCool = 7,
}

/**
 * Get a material preset by name.
 */
export function agentGetMaterial(preset: string): any;

/**
 * Get full color metrics for a hex color.
 */
export function agentGetMetrics(color_hex: string): string;

/**
 * Get metrics for multiple colors in a single WASM call.
 */
export function agentGetMetricsBatch(colors_json: string): string;

/**
 * Improve an existing foreground color against a background.
 */
export function agentImproveForeground(fg_hex: string, bg_hex: string, context: string, target: string): string;

/**
 * List all available materials, optionally filtered by category.
 */
export function agentListMaterials(category?: string | null): string;

/**
 * Recommend a foreground color for a given background.
 */
export function agentRecommendForeground(bg_hex: string, context: string, target: string): string;

/**
 * Score a color pair for quality.
 */
export function agentScorePair(fg_hex: string, bg_hex: string, context: string, target: string): string;

/**
 * Validate a single color against a contract.
 */
export function agentValidate(color_hex: string, contract_json: string): string;

/**
 * Validate a color pair for contrast compliance.
 */
export function agentValidatePair(fg_hex: string, bg_hex: string, standard: number, level: number): string;

/**
 * Validate multiple color pairs in a single WASM call.
 */
export function agentValidatePairsBatch(pairs_json: string): string;

/**
 * Get APCA algorithm constants as JSON.
 */
export function apcaConstants(): any;

/**
 * Apply interpolation mode to a t value (0.0-1.0).
 */
export function applyInterpolation(mode: number, t: number): number;

/**
 * Apply refraction correction to an OKLCH color.
 */
export function applyRefractionToColor(params: RefractionParams, l: number, c: number, h: number, x: number, y: number, incident_angle: number): Float64Array;

/**
 * Apply SIREN correction to OKLCH values.
 */
export function applySirenCorrection(l: number, c: number, h: number, delta_l: number, delta_c: number, delta_h: number): Float64Array;

/**
 * Returns the audio domain name and version as a JSON string.
 *
 * Useful for feature detection from JavaScript.
 *
 * Returns: `{"domain":"audio","name":"momoto-audio","version":"X.Y.Z"}`
 */
export function audioDomainInfo(): string;

/**
 * Compute the one-sided power spectrum of a real-valued mono signal.
 *
 * The input length must be a power of two (e.g. 1024, 2048, 4096).
 * Returns a `Float32Array` of length `N/2 + 1` (DC through Nyquist).
 * Power values are normalised by `1/N` (satisfies Parseval's theorem).
 *
 * Returns an empty array if `samples.len()` is not a power of two or is 0.
 *
 * # JavaScript usage
 * ```javascript
 * const ps = audioFftPowerSpectrum(samples); // samples.length must be 2^k
 * ```
 */
export function audioFftPowerSpectrum(samples: Float32Array): Float32Array;

/**
 * Compute integrated LUFS loudness for a mono signal (one block = 400 ms).
 *
 * Returns integrated loudness in LUFS as f32.
 * Returns `f32::NEG_INFINITY` (as 0.0 in JS) if the signal is silence or
 * falls below the absolute gate (-70 LUFS).
 *
 * # Parameters
 * - `samples`: mono f32 samples (recommended: 400 ms = 19 200 samples at 48 kHz)
 * - `sample_rate`: audio sample rate (44100, 48000, or 96000 Hz)
 *
 * # Returns
 * Integrated LUFS as f32, or -999.0 if sample rate is unsupported.
 */
export function audioLufs(samples: Float32Array, sample_rate: number): number;

/**
 * Compute mel-band energies from a mono signal.
 *
 * Runs FFT + mel filterbank in one call.
 * `samples.len()` must be a power of two.
 * `n_bands` must be > 0.
 *
 * Returns a `Float32Array` of `n_bands` mel-band energy values,
 * or an empty array on invalid input.
 *
 * # Parameters
 * - `samples`: mono f32 samples (power-of-two length)
 * - `sample_rate`: audio sample rate (44100, 48000, 96000)
 * - `n_bands`: number of mel bands (typically 20–128)
 */
export function audioMelSpectrum(samples: Float32Array, sample_rate: number, n_bands: number): Float32Array;

/**
 * Compute momentary LUFS for the most recent 400 ms block.
 *
 * Returns -999.0 for unsupported sample rates.
 */
export function audioMomentaryLufs(samples: Float32Array, sample_rate: number): number;

/**
 * Compute spectral brightness: fraction of power above `threshold_hz`.
 *
 * Returns a value in [0, 1]. Returns 0.0 for silence.
 */
export function audioSpectralBrightness(power_spectrum: Float32Array, sample_rate: number, threshold_hz: number): number;

/**
 * Compute spectral centroid (centre-of-mass of power spectrum) in Hz.
 *
 * `power_spectrum` is the one-sided spectrum returned by `audioFftPowerSpectrum`.
 * Returns 0.0 for silence.
 */
export function audioSpectralCentroid(power_spectrum: Float32Array, sample_rate: number): number;

/**
 * Compute spectral flatness (Wiener entropy): 1.0 = noise-like, 0.0 = tonal.
 */
export function audioSpectralFlatness(power_spectrum: Float32Array): number;

/**
 * Compute spectral flux between two consecutive power spectra.
 *
 * Half-wave rectified (only positive changes). Both slices must have the
 * same length. Returns 0.0 if lengths differ or are empty.
 */
export function audioSpectralFlux(previous: Float32Array, current: Float32Array): number;

/**
 * Compute spectral rolloff: frequency (Hz) below which `roll_percent`×100%
 * of spectral energy lies. Common value: 0.85.
 *
 * Returns 0.0 for silence or invalid `roll_percent`.
 */
export function audioSpectralRolloff(power_spectrum: Float32Array, sample_rate: number, roll_percent: number): number;

/**
 * Validate integrated LUFS against an EBU R128 profile.
 *
 * `profile` must be one of: `"broadcast"`, `"streaming"`, `"podcast"`.
 * Unknown profiles default to the broadcast profile.
 *
 * Returns `true` if the measured loudness is within the permitted range.
 *
 * # JavaScript usage
 * ```javascript
 * const ok = audioValidateEbuR128(-23.0, "broadcast"); // → true
 * const ok2 = audioValidateEbuR128(-10.0, "streaming"); // → false (too loud)
 * ```
 */
export function audioValidateEbuR128(integrated_lufs: number, profile: string): boolean;

/**
 * Fast Beer-Lambert attenuation using lookup table
 *
 * 4x faster than exp() calculation with <1% error.
 *
 * # Arguments
 *
 * * `absorption` - Absorption coefficient per mm (0.0 to 1.0)
 * * `distance` - Path length in mm (0.0 to 100.0)
 *
 * # Returns
 *
 * Transmittance (0.0 to 1.0)
 */
export function beerLambertFast(absorption: number, distance: number): number;

/**
 * Calculate Blinn-Phong specular highlight
 *
 * Uses halfway vector for faster and more accurate specular than Phong model.
 *
 * # Arguments
 *
 * * `normal` - Surface normal vector
 * * `light_dir` - Light direction vector (from surface to light)
 * * `view_dir` - View direction vector (from surface to camera)
 * * `shininess` - Material shininess (1-256, higher = sharper highlight)
 *
 * # Returns
 *
 * Specular intensity (0.0 to 1.0)
 */
export function blinnPhongSpecular(normal: Vec3, light_dir: Vec3, view_dir: Vec3, shininess: number): number;

/**
 * Calculate Brewster's angle (minimum reflectance for p-polarization)
 *
 * # Arguments
 *
 * * `ior1` - Refractive index of first medium
 * * `ior2` - Refractive index of second medium
 *
 * # Returns
 *
 * Brewster's angle in degrees
 */
export function brewsterAngle(ior1: number, ior2: number): number;

/**
 * Calculate ambient shadow CSS string.
 */
export function calculateAmbientShadow(params: AmbientShadowParams, bg_l: number, bg_c: number, bg_h: number, elevation: number): string;

/**
 * Calculate optimal AR coating thickness for a given wavelength.
 *
 * For quarter-wave AR coating: d = lambda / (4 * n_film)
 *
 * # Arguments
 *
 * * `wavelength_nm` - Design wavelength in nanometers (typically 550nm for visible)
 * * `n_film` - Film refractive index
 *
 * # Returns
 *
 * Optimal thickness in nanometers
 *
 * # Example
 *
 * ```javascript
 * // AR coating for green light on MgF2
 * const thickness = calculateArCoatingThickness(550.0, 1.38);
 * console.log(`Optimal thickness: ${thickness}nm`);  // ~99.6nm
 * ```
 */
export function calculateArCoatingThickness(wavelength_nm: number, n_film: number): number;

/**
 * Calculate color shift with viewing angle
 *
 * # Arguments
 * * `film` - TransferMatrixFilm to analyze
 *
 * # Returns
 * Array of { angle: number, rgb: [r, g, b] } objects
 */
export function calculateColorShift(film: TransferMatrixFilm): Array<any>;

/**
 * Calculate contact shadow for a glass element.
 *
 * Generates a sharp, dark shadow at the point where glass meets background.
 *
 * # Arguments
 *
 * * `params` - Contact shadow configuration
 * * `background` - Background color in OKLCH (affects shadow visibility)
 * * `glass_depth` - Perceived thickness of glass (affects shadow intensity, 0.0-2.0)
 *
 * # Returns
 *
 * Calculated contact shadow ready for CSS rendering.
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const params = ContactShadowParams.standard();
 * const background = new OKLCH(0.95, 0.01, 240.0); // Light background
 * const shadow = calculateContactShadow(params, background, 1.0);
 *
 * element.style.boxShadow = shadow.toCss();
 * ```
 */
export function calculateContactShadow(params: ContactShadowParams, background: OKLCH, glass_depth: number): ContactShadow;

/**
 * Calculate elevation shadow for glass element
 *
 * # Arguments
 *
 * * `elevation` - Elevation level (0-24)
 * * `background` - Background color in OKLCH
 * * `glass_depth` - Perceived thickness of glass (0.0-2.0)
 *
 * # Returns
 *
 * Complete shadow system as CSS box-shadow string
 */
export function calculateElevationShadow(elevation: number, background: OKLCH, glass_depth: number): ElevationShadow;

/**
 * Calculate CSS position for highlight from light direction
 *
 * # Arguments
 *
 * * `light_dir` - Light direction vector
 *
 * # Returns
 *
 * Array of [x, y] in percentage (-50 to 150)
 */
export function calculateHighlightPosition(light_dir: Vec3): Float64Array;

/**
 * Calculate interactive shadow for a given state. Returns CSS box-shadow string.
 */
export function calculateInteractiveShadow(transition: ElevationTransition, state: number, bg_l: number, bg_c: number, bg_h: number, glass_depth: number): string;

/**
 * Calculate lighting for a surface. Returns lighting result as JSON.
 */
export function calculateLighting(normal_x: number, normal_y: number, normal_z: number, view_x: number, view_y: number, view_z: number, env: LightingEnvironment, shininess: number): any;

/**
 * Calculate multi-layer transmittance for realistic glass rendering
 *
 * # Arguments
 *
 * * `optical_props` - Optical properties of the glass
 * * `incident_intensity` - Incoming light intensity (0.0-1.0)
 *
 * # Returns
 *
 * Layer-separated transmittance values
 */
export function calculateMultiLayerTransmittance(optical_props: OpticalProperties, incident_intensity: number): LayerTransmittance;

/**
 * Calculate multi-scale ambient shadows (multiple layers). Returns comma-separated CSS.
 */
export function calculateMultiScaleAmbient(params: AmbientShadowParams, bg_l: number, bg_c: number, bg_h: number, elevation: number): string;

/**
 * Calculate refraction at a position with incident angle. Returns [offset_x, offset_y, hue_shift, brightness_factor].
 */
export function calculateRefraction(params: RefractionParams, x: number, y: number, incident_angle: number): Float64Array;

/**
 * Calculate multi-layer specular highlights
 *
 * Generates 4 layers: main, secondary, top edge, left edge
 *
 * # Arguments
 *
 * * `normal` - Surface normal vector
 * * `light_dir` - Light direction vector
 * * `view_dir` - View direction vector
 * * `base_shininess` - Base material shininess
 *
 * # Returns
 *
 * Flat array of [intensity1, x1, y1, size1, intensity2, x2, y2, size2, ...]
 */
export function calculateSpecularLayers(normal: Vec3, light_dir: Vec3, view_dir: Vec3, base_shininess: number): Float64Array;

/**
 * Calculate temperature sensitivity of a Drude metal
 *
 * Returns array of [temp_K, reflectance] pairs showing how
 * reflectance changes with temperature.
 *
 * # Arguments
 * * `metal` - Metal name ("gold", "silver", "copper", etc.)
 * * `wavelength_nm` - Wavelength in nanometers
 *
 * # Returns
 * Array of { temperatureK, reflectance } objects
 */
export function calculateTemperatureSensitivity(metal: string, wavelength_nm: number): Array<any>;

/**
 * Calculate view angle between normal and view direction
 *
 * # Arguments
 *
 * * `normal` - Surface normal vector
 * * `view_dir` - View direction vector
 *
 * # Returns
 *
 * Cosine of angle (for use in Fresnel calculations)
 */
export function calculateViewAngle(normal: Vec3, view_dir: Vec3): number;

/**
 * Get description string for a compliance target.
 */
export function complianceTargetDescription(target: number): string;

/**
 * Compute SIREN neural correction for a foreground/background color pair.
 */
export function computeSirenCorrection(bg_l: number, bg_c: number, bg_h: number, fg_l: number, fg_c: number, fg_h: number, apca_lc: number, wcag_ratio: number, quality: number): SirenCorrection;

/**
 * Batch: Compute SIREN corrections for multiple color pairs.
 */
export function computeSirenCorrectionBatch(inputs: Float64Array): Float64Array;

/**
 * Evaluate Cook-Torrance specular BRDF (GGX + Smith G2 + Schlick Fresnel).
 *
 * # Arguments
 * * `n_dot_v` — cosine of view angle with surface normal (0=grazing, 1=normal)
 * * `n_dot_l` — cosine of light angle with normal
 * * `n_dot_h` — cosine of half-vector with normal
 * * `h_dot_v` — cosine of half-vector with view (for Fresnel)
 * * `roughness` — surface roughness in [0, 1]
 * * `f0` — Fresnel reflectance at normal incidence (0.04 for glass, 0.8+ for metals)
 *
 * # Returns
 *
 * BRDF value ≥ 0. Multiply by `n_dot_l` to get irradiance contribution.
 */
export function cookTorranceBRDF(n_dot_v: number, n_dot_l: number, n_dot_h: number, h_dot_v: number, roughness: number, f0: number): number;

/**
 * Create a color transition sequence.
 */
export function createColorTransition(from_hex: string, to_hex: string, duration_ms: bigint, easing: string, frame_count: number): string;

/**
 * Create a new session and return its ID.
 *
 * # Arguments
 * * `context_json` - Optional JSON string with initial session context
 *
 * # Returns
 * JSON string with session ID and status
 */
export function createSession(context_json?: string | null): string;

/**
 * Compute the perceptual ΔE between a color and its CVD simulation.
 *
 * Higher values = more problematic for the given CVD type.
 * Typical ranges: < 20 = mild, 20–60 = moderate, > 60 = severe.
 */
export function cvdDeltaE(hex: string, cvd_type: string): number;

export function deltaE2000(l1: number, a1: number, b1: number, l2: number, a2: number, b2: number): number;

export function deltaE2000Batch(lab_pairs: Float64Array): Float64Array;

export function deltaE76(l1: number, a1: number, b1: number, l2: number, a2: number, b2: number): number;

export function deltaE94(l1: number, a1: number, b1: number, l2: number, a2: number, b2: number): number;

/**
 * Demonstrate spectral pipeline with different configurations
 *
 * Returns comparison data showing how the same material
 * produces different colors under different conditions.
 */
export function demonstrateSpectralPipeline(): object;

/**
 * Demonstrate stress-optic effect
 *
 * Shows how applied stress changes optical response.
 * Biaxial stress in MPa affects film thickness and therefore color.
 */
export function demonstrateStressOpticEffect(): object;

/**
 * Demonstrate structural color principle
 *
 * Creates a comparison showing that the SAME materials
 * with DIFFERENT structures produce DIFFERENT colors.
 *
 * # Returns
 * Object with two stacks and their resulting colors
 */
export function demonstrateStructuralColor(): object;

/**
 * Demonstrate thermo-optic effect
 *
 * Shows how the SAME material changes color with temperature.
 * This is NOT animation - this is PHYSICS.
 *
 * # Returns
 * Object with cold/room/hot states and their RGB values
 */
export function demonstrateThermoOpticEffect(): object;

/**
 * Derive a gradient from a lighting environment. Returns JSON array.
 */
export function deriveGradient(env: LightingEnvironment, surface_curvature: number, shininess: number, samples: number): any;

/**
 * Compute the perceptual distance between two audio signals.
 *
 * Computes mel spectra for both signals, then returns the L² (Euclidean)
 * distance between them in the 40-band mel feature space.
 *
 * Both signals must have the same length (power of two). Returns 0.0 for
 * identical signals, or -1.0 for invalid/mismatched inputs.
 *
 * # JavaScript usage
 * ```javascript
 * const dist = domainPerceptualDistance(signalA, signalB, 48000);
 * ```
 */
export function domainPerceptualDistance(a: Float32Array, b: Float32Array, sample_rate: number): number;

/**
 * Process a mono signal through the audio domain pipeline.
 *
 * One-shot: applies FFT + mel filterbank and returns `n_bands` mel-band
 * energies as a `Float32Array`. Uses 40 bands and the full 0–Nyquist range.
 *
 * `samples.len()` must be a power of two (minimum 64).
 * Returns empty array on invalid input.
 *
 * # JavaScript usage
 * ```javascript
 * const features = domainProcess(samples, 48000); // → Float32Array(40)
 * ```
 */
export function domainProcess(samples: Float32Array, sample_rate: number): Float32Array;

/**
 * Double Henyey-Greenstein phase function.
 *
 * Two-lobe model for materials with both forward and backward scatter.
 *
 * p(θ) = w × p_HG(θ, g_f) + (1-w) × p_HG(θ, g_b)
 *
 * # Arguments
 *
 * * `cos_theta` - Cosine of scattering angle
 * * `g_forward` - Forward lobe asymmetry (positive)
 * * `g_backward` - Backward lobe asymmetry (negative)
 * * `weight` - Forward lobe weight (0-1)
 */
export function doubleHenyeyGreenstein(cos_theta: number, g_forward: number, g_backward: number, weight: number): number;

export function easeInOut(t: number): number;

/**
 * Calculate edge intensity for edge glow effect
 *
 * # Arguments
 *
 * * `cos_theta` - Cosine of view angle
 * * `edge_power` - Power curve exponent (1.0-4.0, higher = sharper edge)
 *
 * # Returns
 *
 * Edge intensity (0.0 at center to 1.0 at edge)
 */
export function edgeIntensity(cos_theta: number, edge_power: number): number;

export function elevationDp(level: number): number;

export function elevationTintOpacity(level: number): number;

/**
 * Evaluate and render glass material to CSS in one call (convenience function)
 *
 * This is a shortcut for:
 * 1. glass.evaluate(materialContext)
 * 2. backend.render(evaluated, renderContext)
 *
 * # Arguments
 *
 * * `glass` - Glass material to render
 * * `material_context` - Evaluation context (viewing angle, background, etc.)
 * * `render_context` - Rendering context (viewport, pixel ratio, etc.)
 *
 * # Returns
 *
 * CSS string ready to apply to DOM element
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const glass = GlassMaterial.frosted();
 * const materialCtx = EvalMaterialContext.new();
 * const renderCtx = RenderContext.desktop();
 *
 * const css = evaluateAndRenderCss(glass, materialCtx, renderCtx);
 * document.getElementById('panel').style.cssText = css;
 * ```
 */
export function evaluateAndRenderCss(glass: GlassMaterial, material_context: EvalMaterialContext, render_context: RenderContext): string;

/**
 * Batch evaluate and render multiple materials to CSS strings.
 *
 * This is significantly more efficient than calling `evaluateAndRenderCss`
 * in a loop, especially for large numbers of materials.
 *
 * # Arguments
 *
 * * `materials` - Array of GlassMaterial instances
 * * `material_contexts` - Array of EvalMaterialContext instances (same length)
 * * `render_context` - Single RenderContext to use for all materials
 *
 * # Returns
 *
 * Array of CSS strings, one per material
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const materials = [
 *     GlassMaterial.clear(),
 *     GlassMaterial.frosted(),
 *     GlassMaterial.thick()
 * ];
 * const contexts = materials.map(() => EvalMaterialContext.default());
 * const renderCtx = RenderContext.desktop();
 *
 * const cssArray = evaluateAndRenderCssBatch(materials, contexts, renderCtx);
 * cssArray.forEach((css, i) => {
 *     document.getElementById(`panel-${i}`).style.cssText = css;
 * });
 * ```
 */
export function evaluateAndRenderCssBatch(materials: GlassMaterial[], material_contexts: EvalMaterialContext[], render_context: RenderContext): string[];

/**
 * Batch evaluate and render with individual render contexts.
 *
 * More flexible version that allows different render contexts per material.
 *
 * # Arguments
 *
 * * `materials` - Array of GlassMaterial instances
 * * `material_contexts` - Array of EvalMaterialContext instances
 * * `render_contexts` - Array of RenderContext instances (all arrays must match length)
 *
 * # Returns
 *
 * Array of CSS strings, one per material
 */
export function evaluateAndRenderCssBatchFull(materials: GlassMaterial[], material_contexts: EvalMaterialContext[], render_contexts: RenderContext[]): string[];

/**
 * Evaluate a single dielectric material. Returns `[reflectance, transmittance, absorption]`.
 */
export function evaluateDielectricBSDF(ior: number, roughness: number, cos_theta: number): Float64Array;

/**
 * Evaluate multiple materials in a single WASM call for maximum performance.
 *
 * Evaluate a batch of dielectric materials using the full BSDF pipeline.
 *
 * Avoids N×JS↔WASM boundary crossings by processing all materials in one call.
 * Each material is evaluated as a `DielectricBSDF` (Fresnel + roughness).
 *
 * # Arguments
 * * `iors` — index of refraction for each material
 * * `roughnesses` — surface roughness for each material (0=smooth, 1=rough)
 * * `cos_thetas` — cosine of incident angle for each material
 *
 * All three slices must have the same length N.
 *
 * # Returns
 *
 * Flat array of `[reflectance, transmittance, absorption] × N`
 * (length = 3 × N). Energy is always conserved per element.
 */
export function evaluateDielectricBatch(iors: Float64Array, roughnesses: Float64Array, cos_thetas: Float64Array): Float64Array;

/**
 * Evaluate materials in batch. Arrays must be same length.
 */
export function evaluateMaterialBatch(iors: Float64Array, roughnesses: Float64Array, thicknesses: Float64Array, absorptions: Float64Array): any;

/**
 * Evaluate a full microfacet material (Cook-Torrance specular + Oren-Nayar diffuse).
 *
 * # Arguments
 * * `roughness` — surface roughness [0, 1]
 * * `metallic` — metallic factor (0 = dielectric, 1 = metallic)
 * * `f0` — Fresnel at normal incidence
 * * `cos_theta` — incident angle cosine
 *
 * # Returns
 *
 * `[reflectance, transmittance, absorption]` — energy conserved.
 */
export function evaluateMicrofacetBSDF(roughness: number, metallic: number, f0: number, cos_theta: number): Float64Array;

/**
 * Execute a query within a session context.
 *
 * # Arguments
 * * `session_id` - The session ID to use
 * * `query_json` - JSON string with the query to execute
 *
 * # Returns
 * JSON string with query results
 */
export function executeWithSession(session_id: string, query_json: string): string;

/**
 * Execute a workflow from JSON specification.
 *
 * # Arguments
 * * `workflow_json` - JSON string with workflow name or inline definition
 * * `input_json` - JSON string with colors, pairs, and backgrounds
 *
 * # Returns
 * JSON string with workflow execution results
 */
export function executeWorkflow(workflow_json: string, input_json: string): string;

/**
 * Calculate Fresnel F0 (normal incidence reflectance) from IOR.
 *
 * F0 = ((n - 1) / (n + 1))²
 */
export function f0FromIor(ior: number): number;

/**
 * Find the dominant (brightest) wavelength for a thin film.
 *
 * Returns the wavelength with maximum reflectance in the visible range.
 *
 * # Arguments
 *
 * * `film` - ThinFilm parameters
 * * `n_substrate` - Substrate refractive index
 * * `cos_theta` - Cosine of incidence angle
 *
 * # Returns
 *
 * Dominant wavelength in nanometers (400-700nm)
 *
 * # Example
 *
 * ```javascript
 * const film = ThinFilm.soapBubbleMedium();
 * const lambda = findDominantWavelength(film, 1.0, 1.0);
 * console.log(`Dominant color wavelength: ${lambda}nm`);
 * ```
 */
export function findDominantWavelength(film: ThinFilm, n_substrate: number, cos_theta: number): number;

/**
 * Find the peak reflectance wavelength for a film stack
 *
 * # Arguments
 * * `film` - TransferMatrixFilm to analyze
 * * `angle_deg` - Incidence angle in degrees
 *
 * # Returns
 * Wavelength (nm) where reflectance is maximum
 */
export function findPeakWavelength(film: TransferMatrixFilm, angle_deg: number): number;

/**
 * Fast Fresnel calculation using lookup table
 *
 * 5x faster than direct calculation with <1% error.
 * Ideal for batch processing or performance-critical paths.
 *
 * # Arguments
 *
 * * `ior` - Index of refraction (1.0 to 2.5)
 * * `cos_theta` - Cosine of view angle (0.0 to 1.0)
 *
 * # Returns
 *
 * Fresnel reflectance (0.0 to 1.0)
 */
export function fresnelFast(ior: number, cos_theta: number): number;

/**
 * Calculate full Fresnel equations (s and p polarization)
 *
 * More accurate than Schlick's approximation but slower.
 *
 * # Arguments
 *
 * * `ior1` - Refractive index of first medium
 * * `ior2` - Refractive index of second medium
 * * `cos_theta_i` - Cosine of incident angle
 *
 * # Returns
 *
 * Tuple of (Rs, Rp) - reflectance for s and p polarization
 */
export function fresnelFull(ior1: number, ior2: number, cos_theta_i: number): Float64Array;

/**
 * Calculate Fresnel reflectance using Schlick's approximation
 *
 * Fast approximation of angle-dependent reflectivity (<4% error vs full Fresnel).
 *
 * # Arguments
 *
 * * `ior1` - Refractive index of first medium (e.g., 1.0 for air)
 * * `ior2` - Refractive index of second medium (e.g., 1.5 for glass)
 * * `cos_theta` - Cosine of view angle (0 = grazing, 1 = perpendicular)
 *
 * # Returns
 *
 * Reflectance value (0.0 to 1.0)
 */
export function fresnelSchlick(ior1: number, ior2: number, cos_theta: number): number;

/**
 * Generate a distortion map grid. Returns flat array [offset_x, offset_y, hue_shift, brightness, ...].
 */
export function generateDistortionMap(params: RefractionParams, grid_size: number): Float64Array;

/**
 * Generate a complete visual experience from brand colors.
 */
export function generateExperience(preset: string, primary_hex: string, background_hex: string): string;

/**
 * Generate CSS-ready Fresnel gradient
 *
 * # Arguments
 *
 * * `ior` - Index of refraction (e.g., 1.5 for glass)
 * * `samples` - Number of gradient stops (typically 8-16)
 * * `edge_power` - Edge sharpness (1.0-4.0)
 *
 * # Returns
 *
 * Flat array of [position, intensity, position, intensity, ...]
 */
export function generateFresnelGradient(ior: number, samples: number, edge_power: number): Float64Array;

/**
 * Generate Fresnel edge gradient CSS.
 *
 * Creates a radial gradient that simulates angle-dependent reflection
 * (Schlick's approximation). Edges appear brighter than center.
 *
 * # Arguments
 *
 * * `intensity` - Edge glow intensity (0.0-1.0)
 * * `light_mode` - Whether to use light mode colors
 *
 * # Returns
 *
 * CSS radial-gradient string
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const gradient = generateFresnelGradientCss(0.3, true);
 * element.style.background = gradient;
 * ```
 */
export function generateFresnelGradientCss(intensity: number, light_mode: boolean): string;

/**
 * Generate inner top highlight CSS.
 *
 * Creates a linear gradient from top that simulates
 * light hitting the top edge.
 *
 * # Arguments
 *
 * * `intensity` - Highlight intensity (0.0-1.0)
 * * `light_mode` - Whether to use light mode colors
 *
 * # Returns
 *
 * CSS linear-gradient string
 */
export function generateInnerHighlightCss(intensity: number, light_mode: boolean): string;

/**
 * Generate a color palette from an OKLCH seed.
 *
 * # Arguments
 * * `l`, `c`, `h` — seed color in OKLCH
 * * `harmony` — harmony type enum value
 * * `_count` — reserved (harmony type determines count)
 *
 * # Returns
 *
 * Flat array `[L0, C0, H0, L1, C1, H1, ...]` for all generated colors.
 * All colors are gamut-safe.
 */
export function generatePalette(l: number, c: number, h: number, harmony: WasmHarmonyType, _count: number): Float64Array;

/**
 * Generate a palette from a hex seed color.
 *
 * # Arguments
 * * `hex` — seed color as hex string (e.g. "#3a7bd5")
 * * `harmony` — harmony type
 *
 * # Returns
 *
 * Flat `[L, C, H]` array, or empty array if hex is invalid.
 */
export function generatePaletteFromHex(hex: string, harmony: WasmHarmonyType): Float64Array;

/**
 * Generate a report from analysis data.
 *
 * # Arguments
 * * `report_type` - Report type: comprehensive, accessibility, quality, physics
 * * `input_json` - JSON string with colors and pairs to analyze
 * * `format` - Output format: json, markdown, html
 *
 * # Returns
 * Generated report content
 */
export function generateReport(report_type: string, input_json: string, format: string): string;

/**
 * Generate secondary specular (fill light) CSS.
 *
 * Creates a weaker highlight at bottom-right to simulate
 * ambient/fill lighting.
 *
 * # Arguments
 *
 * * `intensity` - Highlight intensity (0.0-1.0)
 * * `size` - Highlight size as percentage (15-40)
 *
 * # Returns
 *
 * CSS radial-gradient string
 */
export function generateSecondarySpecularCss(intensity: number, size: number): string;

/**
 * Generate tonal shades for a base color.
 *
 * Returns flat array `[L0, C0, H0, ..., L9, C9, H9]` for 10 shades
 * (lightness descending: ~0.97 to ~0.12).
 */
export function generateShades(l: number, c: number, h: number): Float64Array;

/**
 * Generate specular highlight CSS.
 *
 * Creates a positioned radial gradient for light reflection
 * based on Blinn-Phong model.
 *
 * # Arguments
 *
 * * `intensity` - Highlight intensity (0.0-1.0)
 * * `size` - Highlight size as percentage (20-60)
 * * `pos_x` - Horizontal position percentage (0-100)
 * * `pos_y` - Vertical position percentage (0-100)
 *
 * # Returns
 *
 * CSS radial-gradient string
 */
export function generateSpecularHighlightCss(intensity: number, size: number, pos_x: number, pos_y: number): string;

/**
 * Get all advanced thin-film preset names and descriptions
 *
 * # Returns
 * Array of { name: string, layerCount: number } objects
 */
export function getAdvancedThinFilmPresets(): Array<any>;

/**
 * Get default spectral sampling wavelengths (31 points, 380-780nm)
 */
export function getDefaultSpectralSampling(): Float64Array;

/**
 * Get dispersion wavelength constants.
 *
 * # Returns
 *
 * Object with standard wavelengths { red, green, blue, sodiumD, visibleMin, visibleMax }
 */
export function getDispersionWavelengths(): object;

/**
 * Get all Drude metal presets with temperature capability.
 */
export function getDrudeMetalPresets(): Array<any>;

/**
 * Get memory usage for dynamic optics
 */
export function getDynamicOpticsMemory(): number;

/**
 * Get all enhanced glass presets as JSON array.
 */
export function getEnhancedGlassPresets(): any;

/**
 * Get high-resolution spectral sampling (81 points, 380-780nm)
 */
export function getHighResSpectralSampling(): Float64Array;

/**
 * Get all metal presets with their names.
 *
 * # Returns
 *
 * Array of objects with { name, nRgb, kRgb, f0Rgb }
 */
export function getMetalPresets(): Array<any>;

/**
 * Get all dynamic (polydisperse) presets.
 */
export function getMieDynamicPresets(): Array<any>;

/**
 * Get memory usage of Mie LUT.
 */
export function getMieLutMemory(): number;

/**
 * Get all particle presets with their names and properties.
 */
export function getMieParticlePresets(): Array<any>;

/**
 * Get the Momoto system identity and version.
 */
export function getMomotoIdentity(): string;

/**
 * Get all oxidized metal presets
 */
export function getOxidizedMetalPresets(): Array<any>;

/**
 * Get presets by quality tier. Tier: "fast", "standard", "high", "ultra_high", "experimental", "reference".
 */
export function getPresetsByQuality(tier: string): any;

/**
 * Get RGB-only sampling wavelengths (3 points)
 */
export function getRgbSampling(): Float64Array;

/**
 * Get all thin-film presets with their names and recommended substrates.
 *
 * # Returns
 *
 * Array of objects with { name, nFilm, thicknessNm, suggestedSubstrate }
 *
 * # Example
 *
 * ```javascript
 * const presets = getThinFilmPresets();
 * for (const preset of presets) {
 *     console.log(`${preset.name}: n=${preset.nFilm}, d=${preset.thicknessNm}nm`);
 * }
 * ```
 */
export function getThinFilmPresets(): Array<any>;

/**
 * Get transfer matrix memory usage
 */
export function getTransferMatrixMemory(): number;

/**
 * Evaluate GGX Normal Distribution Function.
 *
 * # Arguments
 * * `n_dot_h` — cosine of half-vector with normal
 * * `roughness` — surface roughness (alpha = roughness²)
 *
 * # Returns
 *
 * NDF value (unnormalised density). Peaks at n·h = 1.0.
 */
export function ggxNDF(n_dot_h: number, roughness: number): number;

/**
 * Convert a lighting gradient to CSS stops. Returns JSON array of [{position, l, c, h}, ...].
 */
export function gradientToCss(env: LightingEnvironment, surface_curvature: number, shininess: number, samples: number, base_l: number, base_c: number, base_h: number): any;

/**
 * Compute harmony score for a palette.
 *
 * # Arguments
 * * `lch_flat` — flat array `[L0, C0, H0, L1, C1, H1, ...]`
 *
 * # Returns
 *
 * Score in [0, 1] — higher is more harmonious.
 */
export function harmonyScore(lch_flat: Float64Array): number;

/**
 * Get the maximum achievable chroma for a given hue and tone.
 *
 * Useful for building a UI slider that shows the valid chroma range.
 */
export function hctMaxChroma(hue: number, tone: number): number;

/**
 * Convert HCT components to a hex color string.
 *
 * Chroma may be clamped to the sRGB gamut boundary.
 */
export function hctToHex(hue: number, chroma: number, tone: number): string;

/**
 * Convert HCT to OKLCH flat array `[L, C, H]`.
 */
export function hctToOklch(hue: number, chroma: number, tone: number): Float64Array;

/**
 * Generate a tonal palette in HCT space.
 *
 * Returns flat array `[H0, C0, T0, H1, C1, T1, ...]` for tones:
 * 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100 (13 steps).
 */
export function hctTonalPalette(hue: number, chroma: number): Float64Array;

/**
 * Henyey-Greenstein phase function.
 *
 * Standard phase function for volumetric scattering.
 *
 * p(θ) = (1 - g²) / (4π × (1 + g² - 2g×cosθ)^1.5)
 *
 * # Arguments
 *
 * * `cos_theta` - Cosine of scattering angle (-1 to 1)
 * * `g` - Asymmetry parameter (-1 to 1)
 *
 * # Properties
 *
 * - g = 0: Isotropic (Rayleigh-like)
 * - g > 0: Forward scattering (typical for aerosols)
 * - g < 0: Backward scattering
 */
export function henyeyGreenstein(cos_theta: number, g: number): number;

/**
 * Convert a hex color string to HCT flat array `[hue, chroma, tone]`.
 *
 * Returns `[0, 0, 0]` if the hex string is invalid.
 */
export function hexToHct(hex: string): Float64Array;

/**
 * Convert hex string to OKLCH flat array `[L, C, H]`, or empty on error.
 */
export function hexToOklch(hex: string): Float64Array;

export function init(): void;

/**
 * Interpolate between two values using a mode.
 */
export function interpolateValues(mode: number, a: number, b: number, t: number): number;

/**
 * Check if text qualifies as "large text" per WCAG.
 */
export function isLargeText(font_size_px: number, font_weight: number): boolean;

export function labToRgb(l: number, a: number, b: number): Float64Array;

/**
 * sRGB gamma encoding.
 */
export function linearToSrgb(value: number): number;

/**
 * List available preset workflows.
 *
 * # Returns
 * JSON array of workflow names and descriptions
 */
export function listWorkflows(): string;

/**
 * Get effective specular intensity.
 */
export function materialEffectiveSpecular(material: EvaluatedMaterial): number;

/**
 * Check if an evaluated material has subsurface scattering.
 */
export function materialHasScattering(material: EvaluatedMaterial): boolean;

/**
 * Check if an evaluated material is emissive.
 */
export function materialIsEmissive(material: EvaluatedMaterial): boolean;

/**
 * Check if an evaluated material is transparent.
 */
export function materialIsTransparent(material: EvaluatedMaterial): boolean;

/**
 * Convert a dielectric material to its dominant OKLCH color via spectral integration.
 *
 * Evaluates the material's BSDF at 31 wavelengths (400–700 nm, 10 nm steps),
 * weights by the D65 illuminant, integrates with CIE 1931 2-degree CMFs,
 * and converts to OKLCH.
 *
 * IOR dispersion is modeled via Cauchy: n(λ) = n₀ + 0.004/λ² (λ in μm).
 *
 * # Arguments
 * * `ior` — base IOR at 589 nm (sodium D line)
 * * `roughness` — surface roughness in [0, 1]
 * * `cos_theta` — cosine of incidence angle
 *
 * # Returns
 *
 * Flat array `[L, C, H, reflectance, cct]` where:
 * - L, C, H are OKLCH components of the dominant color
 * - reflectance: spectrally-averaged reflectance (0–1)
 * - cct: Correlated Color Temperature in Kelvin (McCamy 1992)
 */
export function materialToDominantColor(ior: number, roughness: number, cos_theta: number): Float64Array;

/**
 * Inverse linear interpolation.
 */
export function mathInverseLerp(a: number, b: number, value: number): number;

/**
 * Linear interpolation.
 */
export function mathLerp(a: number, b: number, t: number): number;

/**
 * Convert OKLCH to HCT flat array `[hue, chroma, tone]`.
 */
export function oklchToHct(l: number, c: number, h: number): Float64Array;

/**
 * Convert OKLCH to hex string.
 */
export function oklchToHex(l: number, c: number, h: number): string;

/**
 * Evaluate Oren-Nayar diffuse BRDF for rough surfaces.
 *
 * # Arguments
 * * `n_dot_l` — cosine of light angle with normal
 * * `n_dot_v` — cosine of view angle with normal
 * * `l_dot_v` — cosine of angle between light and view directions
 * * `roughness` — surface roughness (0 = Lambertian, 1 = fully rough)
 *
 * # Returns
 *
 * BRDF value ≥ 0 (includes 1/π normalisation). At roughness=0 returns 1/π (Lambert).
 */
export function orenNayarBRDF(n_dot_l: number, n_dot_v: number, l_dot_v: number, roughness: number): number;

/**
 * Calculate quarter-wave thickness for a material
 *
 * # Arguments
 * * `n` - Refractive index
 * * `design_lambda` - Design wavelength in nm
 *
 * # Returns
 * Thickness in nm for quarter-wave optical path
 */
export function quarterWaveThickness(n: number, design_lambda: number): number;

/**
 * Rayleigh scattering efficiency.
 *
 * Q_sca = (8/3) × x⁴ × |((m²-1)/(m²+2))|²
 */
export function rayleighEfficiency(size_param: number, relative_ior: number): number;

/**
 * Rayleigh RGB intensity (wavelength-dependent).
 *
 * Shows λ⁻⁴ blue enhancement.
 */
export function rayleighIntensityRgb(cos_theta: number): Float64Array;

/**
 * Rayleigh phase function.
 *
 * For particles much smaller than wavelength (x << 1).
 *
 * p(θ) = (3/4) × (1 + cos²θ)
 *
 * # Properties
 *
 * - Symmetric (equal forward/backward)
 * - Responsible for blue sky (λ⁻⁴ dependence)
 */
export function rayleighPhase(cos_theta: number): number;

/**
 * Recommend foreground colors for multiple backgrounds in a single WASM call.
 */
export function recommendForegroundBatch(backgrounds: Uint8Array): Array<any>;

/**
 * Calculate APCA relative luminance.
 */
export function relativeLuminanceApca(color: Color): number;

/**
 * Calculate WCAG relative luminance for multiple colors.
 */
export function relativeLuminanceBatch(rgb_data: Uint8Array): Float64Array;

/**
 * Calculate WCAG 2.1 relative luminance.
 */
export function relativeLuminanceSrgb(color: Color): number;

export function remap(value: number, in_min: number, in_max: number, out_min: number, out_max: number): number;

/**
 * Render enhanced CSS for an evaluated material with a render config.
 */
export function renderEnhancedCss(material: EvaluatedMaterial, context: CssRenderConfig): string;

/**
 * Render enhanced glass CSS with physics-based effects.
 *
 * This generates complete CSS with:
 * - Multi-layer backgrounds with gradients
 * - Specular highlights (Blinn-Phong)
 * - Fresnel edge glow
 * - 4-layer elevation shadows
 * - Backdrop blur with saturation
 *
 * # Example (JavaScript)
 *
 * ```javascript
 * const glass = GlassMaterial.regular();
 * const ctx = EvalMaterialContext.new();
 * const rctx = RenderContext.desktop();
 * const options = GlassRenderOptions.premium();
 *
 * const css = renderEnhancedGlassCss(glass, ctx, rctx, options);
 * document.getElementById('panel').style.cssText = css;
 * ```
 */
export function renderEnhancedGlassCss(glass: GlassMaterial, material_context: EvalMaterialContext, _render_context: RenderContext, options: GlassRenderOptions): string;

/**
 * Render premium CSS with default config.
 */
export function renderPremiumCss(material: EvaluatedMaterial): string;

export function rgbToLab(r: number, g: number, b: number): Float64Array;

/**
 * Convert PBR roughness to Blinn-Phong shininess
 *
 * Maps roughness (0.0-1.0) to shininess (1-256) using perceptually linear curve.
 *
 * # Arguments
 *
 * * `roughness` - Surface roughness (0.0 = smooth, 1.0 = rough)
 *
 * # Returns
 *
 * Shininess value for Blinn-Phong (1-256)
 */
export function roughnessToShininess(roughness: number): number;

/**
 * Generate a roughness variation field for procedural surface micro-texture.
 *
 * # Arguments
 * * `base_roughness` — central roughness (0=mirror, 1=fully diffuse)
 * * `variation` — maximum deviation
 * * `cols`, `rows` — grid dimensions
 * * `seed` — noise seed
 *
 * # Returns
 *
 * Flat array clamped to `[0, 1]`.
 */
export function roughnessVariationField(base_roughness: number, variation: number, cols: number, rows: number, seed: number): Float64Array;

/**
 * Calculate scattering color based on particle size.
 *
 * Demonstrates the key principle:
 * - Small particles (Rayleigh): Blue scattering
 * - Large particles (Geometric): White/gray scattering
 *
 * # Arguments
 *
 * * `radius_um` - Particle radius in micrometers
 * * `n_particle` - Particle refractive index
 *
 * # Returns
 *
 * Object { r, g, b, regime, explanation }
 */
export function scatteringColorFromRadius(radius_um: number, n_particle: number): object;

/**
 * Score multiple (fg, bg) color pairs in a single WASM call.
 */
export function scorePairsBatch(pairs: Float64Array): Float64Array;

/**
 * Run self-certification to verify the Momoto engine integrity.
 */
export function selfCertify(): string;

/**
 * Simulate how a hex color appears to a dichromat.
 *
 * # Arguments
 * * `hex` — input color as hex string (e.g. "#ff0000")
 * * `cvd_type` — "protanopia", "deuteranopia", or "tritanopia"
 *
 * # Returns
 *
 * Simulated hex string (what the dichromat perceives), or original hex on error.
 */
export function simulateCVD(hex: string, cvd_type: string): string;

/**
 * Simulate CVD for an OKLCH color. Returns flat `[L, C, H]` of simulated color.
 */
export function simulateCVDOklch(l: number, c: number, h: number, cvd_type: string): Float64Array;

/**
 * Get network metadata as JSON.
 */
export function sirenMetadata(): any;

/**
 * Export raw network weights for inspection/debugging.
 */
export function sirenWeights(): any;

export function smootherstep(t: number): number;

export function smoothstep(t: number): number;

/**
 * APCA soft-clamp function.
 */
export function softClamp(y: number, threshold: number, exponent: number): number;

/**
 * Solve a set of color constraints for a palette.
 *
 * # Arguments
 * * `lch_flat` — flat `[L0, C0, H0, L1, C1, H1, ...]` palette in OKLCH
 * * `constraints_json` — JSON array of constraint specs (see format below)
 * * `max_iterations` — override max iterations (0 = use default 500)
 *
 * # Constraint JSON format
 * ```json
 * [
 *   {"colorIdx":0,"kind":"MinContrast","otherIdx":1,"target":4.5},
 *   {"colorIdx":0,"kind":"MinAPCA","otherIdx":1,"target":60.0},
 *   {"colorIdx":0,"kind":"HarmonyAngle","otherIdx":1,"expectedDeltaH":180,"tolerance":5},
 *   {"colorIdx":0,"kind":"InGamut"},
 *   {"colorIdx":0,"kind":"LightnessRange","min":0.7,"max":1.0},
 *   {"colorIdx":0,"kind":"ChromaRange","min":0.0,"max":0.2}
 * ]
 * ```
 *
 * # Returns
 * JSON `{colors:[L,C,H,...], converged:bool, iterations:number, finalPenalty:number, violations:[...]}`
 */
export function solveColorConstraints(lch_flat: Float64Array, constraints_json: string, max_iterations: number): any;

/**
 * sRGB gamma decoding.
 */
export function srgbToLinear(value: number): number;

/**
 * Generate a warm or cool palette.
 *
 * Returns flat array `[L0, C0, H0, ...]` for 5 colors.
 */
export function temperaturePalette(warm: boolean): Float64Array;

/**
 * Evaluate a drying-paint temporal material at a given time.
 *
 * Returns `[reflectance, transmittance, absorption]`.
 */
export function temporalDryingPaint(t: number, cos_theta: number): Float64Array;

/**
 * Evaluate a soap-bubble thin-film at a given time.
 *
 * Returns `[reflectance, transmittance, absorption]`.
 */
export function temporalSoapBubble(t: number, cos_theta: number): Float64Array;

/**
 * Generate CSS for a Bragg mirror at a specific design wavelength
 *
 * # Arguments
 * * `design_lambda` - Design wavelength in nm
 *
 * # Returns
 * CSS radial-gradient string
 */
export function toCssBraggMirror(design_lambda: number): string;

/**
 * Get total LUT memory usage in bytes
 */
export function totalLutMemory(): number;

/**
 * Get minimum APCA Lc for a usage context.
 */
export function usageMinApcaLc(usage: number): number;

/**
 * Get minimum WCAG AA contrast ratio for a usage context.
 */
export function usageMinWcagAA(usage: number): number;

/**
 * Get minimum WCAG AAA contrast ratio for a usage context.
 */
export function usageMinWcagAAA(usage: number): number;

/**
 * Whether this usage context requires compliance checking.
 */
export function usageRequiresCompliance(usage: number): boolean;

/**
 * Generate a 2D IOR variation field for procedural material texturing.
 *
 * Models micro-scale IOR variation across a glass surface, useful for
 * frosted-glass distortion maps.
 *
 * # Arguments
 * * `base_ior` — central IOR value (e.g. 1.5 for glass)
 * * `variation` — maximum deviation from base IOR (e.g. 0.05)
 * * `cols` — width in samples
 * * `rows` — height in samples
 * * `seed` — noise seed for reproducibility
 *
 * # Returns
 *
 * Flat array of `cols * rows` IOR values in `[base_ior - variation, base_ior + variation]`.
 */
export function variationField(base_ior: number, variation: number, cols: number, rows: number, seed: number): Float64Array;

/**
 * Calculate WCAG contrast ratio directly from two colors.
 */
export function wcagContrastRatio(fg: Color, bg: Color): number;

/**
 * Calculate WCAG contrast ratios for multiple pairs.
 */
export function wcagContrastRatioBatch(pairs: Uint8Array): Float64Array;

/**
 * Determine the highest WCAG level achieved.
 */
export function wcagLevel(ratio: number, is_large: boolean): number;

/**
 * Check if a contrast ratio passes a specific WCAG level.
 */
export function wcagPasses(ratio: number, level: number, is_large: boolean): boolean;

/**
 * Get the minimum required contrast ratio for a WCAG level + text size.
 */
export function wcagRequirement(level: number, is_large: boolean): number;

/**
 * Get WCAG requirements matrix as flat array.
 */
export function wcagRequirementsMatrix(): Float64Array;
