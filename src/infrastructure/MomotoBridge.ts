// =============================================================================
// MomotoBridge.ts — Unified TypeScript Bridge to Momoto WASM
//
// RULES:
// 1. ZERO algorithmic logic — all math runs in Rust/WASM
// 2. Thin wrappers — 1 Rust function = 1 TS call
// 3. Type adaptation only — map TS types to WASM types
// 4. Lazy initialization — WASM loads on first use
// 5. Batch-first — prefer batch APIs to minimize call overhead
// =============================================================================

// --- WASM Module Type (auto-generated from wasm-bindgen) ---
type WasmModule = typeof import('momoto-wasm');

let _wasm: WasmModule | null = null;
let _initPromise: Promise<WasmModule> | null = null;
let _initError: Error | null = null;

// =============================================================================
// Initialization
// =============================================================================

async function loadWasm(): Promise<WasmModule> {
  if (_wasm) return _wasm;
  if (_initPromise) return _initPromise;

  _initPromise = (async () => {
    try {
      const mod = await import('momoto-wasm');
      // `mod.default` is the wasm-pack init function in --target bundler/web formats.
      // In static-import formats (e.g. Cloudflare-compiled output), the WASM is already
      // initialized at module evaluation time and `default` may not be a function.
      if (typeof mod.default === 'function') {
        await mod.default(); // Initialize WASM memory
      }
      _wasm = mod;
      return mod;
    } catch (e) {
      _initError = e instanceof Error ? e : new Error(String(e));
      throw _initError;
    }
  })();

  return _initPromise;
}

function wasm(): WasmModule {
  if (!_wasm) throw new Error('MomotoBridge not initialized. Call MomotoBridge.init() first.');
  return _wasm;
}

// =============================================================================
// Public API: MomotoBridge
// =============================================================================

export const MomotoBridge = {
  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  /** Initialize the WASM module. Safe to call multiple times. */
  async init(): Promise<void> {
    await loadWasm();
  },

  /** Check if WASM is loaded and ready. */
  isReady(): boolean {
    return _wasm !== null;
  },

  /** Get initialization error, if any. */
  getError(): Error | null {
    return _initError;
  },

  /** Assert WASM is initialized (throws if not). */
  assertReady(): void {
    if (!_wasm) throw new Error('MomotoBridge not initialized. Call MomotoBridge.init() first.');
  },

  /** Reset for testing only. */
  __resetForTests(): void {
    _wasm = null;
    _initPromise = null;
    _initError = null;
  },

  // ---------------------------------------------------------------------------
  // Color (delegates to Rust momoto-core::Color)
  // ---------------------------------------------------------------------------

  color: {
    fromHex(hex: string) {
      return wasm().Color.fromHex(hex);
    },
    fromRgb(r: number, g: number, b: number) {
      return new (wasm().Color)(r, g, b);
    },
    toHex(color: InstanceType<WasmModule['Color']>): string {
      return color.toHex();
    },
    lighten(color: InstanceType<WasmModule['Color']>, amount: number) {
      return color.lighten(amount);
    },
    darken(color: InstanceType<WasmModule['Color']>, amount: number) {
      return color.darken(amount);
    },
    saturate(color: InstanceType<WasmModule['Color']>, amount: number) {
      return color.saturate(amount);
    },
    desaturate(color: InstanceType<WasmModule['Color']>, amount: number) {
      return color.desaturate(amount);
    },
  },

  // ---------------------------------------------------------------------------
  // OKLCH (delegates to Rust momoto-core::OKLCH)
  // ---------------------------------------------------------------------------

  oklch: {
    create(l: number, c: number, h: number) {
      return new (wasm().OKLCH)(l, c, h);
    },
    fromColor(color: InstanceType<WasmModule['Color']>) {
      return wasm().OKLCH.fromColor(color);
    },
    toColor(oklch: InstanceType<WasmModule['OKLCH']>) {
      return oklch.toColor();
    },
    interpolate(
      a: InstanceType<WasmModule['OKLCH']>,
      b: InstanceType<WasmModule['OKLCH']>,
      t: number,
      huePath: 'shorter' | 'longer' = 'shorter',
    ) {
      return wasm().OKLCH.interpolate(a, b, t, huePath);
    },
    deltaE(
      a: InstanceType<WasmModule['OKLCH']>,
      b: InstanceType<WasmModule['OKLCH']>,
    ): number {
      return a.deltaE(b);
    },
    mapToGamut(oklch: InstanceType<WasmModule['OKLCH']>) {
      return oklch.mapToGamut();
    },
  },

  // ---------------------------------------------------------------------------
  // OKLab (delegates to Rust momoto-core::OKLab)
  // ---------------------------------------------------------------------------

  oklab: {
    create(l: number, a: number, b: number) {
      return new (wasm().OKLab)(l, a, b);
    },
    fromColor(color: InstanceType<WasmModule['Color']>) {
      return wasm().OKLab.fromColor(color);
    },
    toColor(oklab: InstanceType<WasmModule['OKLab']>) {
      return oklab.toColor();
    },
    toOklch(oklab: InstanceType<WasmModule['OKLab']>) {
      return oklab.toOklch();
    },
    interpolate(
      from: InstanceType<WasmModule['OKLab']>,
      to: InstanceType<WasmModule['OKLab']>,
      t: number,
    ) {
      return wasm().OKLab.interpolate(from, to, t);
    },
    deltaE(
      a: InstanceType<WasmModule['OKLab']>,
      b: InstanceType<WasmModule['OKLab']>,
    ): number {
      return a.deltaE(b);
    },
  },

  // ---------------------------------------------------------------------------
  // Contrast Metrics (delegates to Rust momoto-metrics)
  // ---------------------------------------------------------------------------

  contrast: {
    /** Full WCAG 2.1 evaluation. */
    wcag(fg: InstanceType<WasmModule['Color']>, bg: InstanceType<WasmModule['Color']>) {
      const metric = new (wasm().WCAGMetric)();
      const result = metric.evaluate(fg, bg);
      metric.free();
      return result;
    },

    /** Full APCA evaluation with polarity. */
    apca(fg: InstanceType<WasmModule['Color']>, bg: InstanceType<WasmModule['Color']>) {
      const metric = new (wasm().APCAMetric)();
      const result = metric.evaluate(fg, bg);
      metric.free();
      return result;
    },

    /** Direct WCAG contrast ratio (faster than full evaluate). */
    wcagRatio(fg: InstanceType<WasmModule['Color']>, bg: InstanceType<WasmModule['Color']>): number {
      return wasm().wcagContrastRatio(fg, bg);
    },

    /** Check if ratio passes WCAG level. */
    wcagPasses(ratio: number, level: 'AA' | 'AAA', isLarge: boolean): boolean {
      return wasm().wcagPasses(ratio, level === 'AAA' ? 1 : 0, isLarge);
    },

    /** Determine highest WCAG level: null | 'AA' | 'AAA'. */
    wcagLevel(ratio: number, isLarge: boolean): null | 'AA' | 'AAA' {
      const level = wasm().wcagLevel(ratio, isLarge);
      if (level === 0) return null;
      if (level === 1) return 'AA';
      return 'AAA';
    },

    /** Check if text qualifies as "large text" per WCAG. */
    isLargeText(fontSizePx: number, fontWeight: number): boolean {
      return wasm().isLargeText(fontSizePx, fontWeight);
    },

    /** Get WCAG minimum ratio for level + text size. */
    wcagRequirement(level: 'AA' | 'AAA', isLarge: boolean): number {
      return wasm().wcagRequirement(level === 'AAA' ? 1 : 0, isLarge);
    },

    /** Get full requirements matrix: [AA_normal, AA_large, AAA_normal, AAA_large]. */
    wcagRequirementsMatrix(): Float64Array {
      return new Float64Array(wasm().wcagRequirementsMatrix());
    },

    /** Get APCA algorithm constants (reference values). */
    apcaConstants(): Record<string, number> {
      return wasm().apcaConstants();
    },

    /** Batch: WCAG contrast ratios for multiple pairs. */
    wcagRatioBatch(pairs: Uint8Array): Float64Array {
      return new Float64Array(wasm().wcagContrastRatioBatch(pairs));
    },

    /** Batch: WCAG relative luminance for multiple colors. */
    luminanceBatch(rgbData: Uint8Array): Float64Array {
      return new Float64Array(wasm().relativeLuminanceBatch(rgbData));
    },
  },

  // ---------------------------------------------------------------------------
  // Luminance (delegates to Rust momoto-core::luminance)
  // ---------------------------------------------------------------------------

  luminance: {
    srgb(color: InstanceType<WasmModule['Color']>): number {
      return wasm().relativeLuminanceSrgb(color);
    },
    apca(color: InstanceType<WasmModule['Color']>): number {
      return wasm().relativeLuminanceApca(color);
    },
    srgbToLinear(value: number): number {
      return wasm().srgbToLinear(value);
    },
    linearToSrgb(value: number): number {
      return wasm().linearToSrgb(value);
    },
  },

  // ---------------------------------------------------------------------------
  // Quality Scoring (delegates to Rust momoto-intelligence)
  // ---------------------------------------------------------------------------

  quality: {
    /** Score a color pair for overall quality. */
    score(
      fg: InstanceType<WasmModule['Color']>,
      bg: InstanceType<WasmModule['Color']>,
      usage: number,
      target: number,
    ) {
      const ctx = new (wasm().RecommendationContext)(usage, target);
      const scorer = new (wasm().QualityScorer)();
      const score = scorer.score(fg, bg, ctx);
      scorer.free();
      ctx.free();
      return score;
    },

    /** Batch: Score multiple pairs. */
    scoreBatch(pairs: Float64Array): Float64Array {
      return new Float64Array(wasm().scorePairsBatch(pairs));
    },

    /** Get minimum WCAG AA ratio for usage context. */
    minWcagAA(usage: number): number {
      return wasm().usageMinWcagAA(usage);
    },

    /** Get minimum APCA Lc for usage context. */
    minApcaLc(usage: number): number {
      return wasm().usageMinApcaLc(usage);
    },

    /** Whether usage context requires compliance. */
    requiresCompliance(usage: number): boolean {
      return wasm().usageRequiresCompliance(usage);
    },
  },

  // ---------------------------------------------------------------------------
  // Recommendations (delegates to Rust momoto-intelligence)
  // ---------------------------------------------------------------------------

  recommend: {
    /** Recommend optimal foreground for a background. */
    foreground(
      bg: InstanceType<WasmModule['Color']>,
      usage: number,
      target: number,
    ) {
      const engine = new (wasm().RecommendationEngine)();
      const rec = engine.recommendForeground(bg, usage, target);
      engine.free();
      return rec;
    },

    /** Improve existing foreground against background. */
    improveForeground(
      fg: InstanceType<WasmModule['Color']>,
      bg: InstanceType<WasmModule['Color']>,
      usage: number,
      target: number,
    ) {
      const engine = new (wasm().RecommendationEngine)();
      const rec = engine.improveForeground(fg, bg, usage, target);
      engine.free();
      return rec;
    },

    /** Batch: Recommend foregrounds for multiple backgrounds. */
    foregroundBatch(backgrounds: Uint8Array) {
      return wasm().recommendForegroundBatch(backgrounds);
    },
  },

  // ---------------------------------------------------------------------------
  // Explanations (delegates to Rust momoto-intelligence)
  // ---------------------------------------------------------------------------

  explain: {
    contrastImprovement(
      originalHex: string,
      recommendedHex: string,
      backgroundHex: string,
      originalRatio: number,
      newRatio: number,
      targetRatio: number,
      deltaL: number,
      deltaC: number,
      deltaH: number,
    ) {
      const gen = new (wasm().ExplanationGenerator)();
      const explanation = gen.explainContrastImprovement(
        originalHex, recommendedHex, backgroundHex,
        originalRatio, newRatio, targetRatio,
        deltaL, deltaC, deltaH,
      );
      gen.free();
      return explanation;
    },
  },

  // ---------------------------------------------------------------------------
  // Advanced Scoring (delegates to Rust momoto-intelligence)
  // ---------------------------------------------------------------------------

  advancedScoring: {
    scoreRecommendation(
      category: string,
      before: QualityScoreInput,
      after: QualityScoreInput,
      deltaL: number,
      deltaC: number,
      deltaH: number,
    ) {
      const scorer = new (wasm().AdvancedScorer)();
      const result = scorer.scoreRecommendation(
        category,
        before.overall, before.compliance, before.perceptual, before.appropriateness,
        after.overall, after.compliance, after.perceptual, after.appropriateness,
        deltaL, deltaC, deltaH,
      );
      scorer.free();
      return result;
    },
  },

  // ---------------------------------------------------------------------------
  // Convergence Detection (delegates to Rust momoto-intelligence)
  // ---------------------------------------------------------------------------

  convergence: {
    create(preset: 'default' | 'fast' | 'highQuality' | 'neural' = 'default') {
      switch (preset) {
        case 'fast': return wasm().ConvergenceDetector.fast();
        case 'highQuality': return wasm().ConvergenceDetector.highQuality();
        case 'neural': return wasm().ConvergenceDetector.neural();
        default: return new (wasm().ConvergenceDetector)();
      }
    },
  },

  // ---------------------------------------------------------------------------
  // Agent (delegates to Rust momoto-agent)
  // ---------------------------------------------------------------------------

  agent: {
    /** Create an agent executor for query/response operations. */
    createExecutor() {
      return new (wasm().AgentExecutor)();
    },

    /** Quick: Validate a color pair. */
    validatePair(fgHex: string, bgHex: string, standard: 'wcag' | 'apca', level: 'AA' | 'AAA'): string {
      return wasm().agentValidatePair(
        fgHex, bgHex,
        standard === 'apca' ? 1 : 0,
        level === 'AAA' ? 1 : 0,
      );
    },

    /** Quick: Get color metrics. */
    getMetrics(hex: string): string {
      return wasm().agentGetMetrics(hex);
    },

    /** Quick: Recommend foreground. */
    recommendForeground(bgHex: string, context: string, target: string): string {
      return wasm().agentRecommendForeground(bgHex, context, target);
    },

    /** Quick: Improve foreground. */
    improveForeground(fgHex: string, bgHex: string, context: string, target: string): string {
      return wasm().agentImproveForeground(fgHex, bgHex, context, target);
    },

    /** Quick: Score a pair. */
    scorePair(fgHex: string, bgHex: string, context: string, target: string): string {
      return wasm().agentScorePair(fgHex, bgHex, context, target);
    },

    /** Batch: Validate multiple pairs. */
    validatePairsBatch(pairs: Array<{ fg: string; bg: string; standard: number; level: number }>): string {
      return wasm().agentValidatePairsBatch(JSON.stringify(pairs));
    },

    /** Batch: Get metrics for multiple colors. */
    getMetricsBatch(hexColors: string[]): string {
      return wasm().agentGetMetricsBatch(JSON.stringify(hexColors));
    },

    /** Build a contract with constraints. */
    contract() {
      return new (wasm().ContractBuilder)();
    },

    /** Generate a complete visual experience. */
    generateExperience(preset: string, primaryHex: string, backgroundHex: string): string {
      return wasm().generateExperience(preset, primaryHex, backgroundHex);
    },

    /** Get Momoto system identity. */
    getIdentity(): string {
      return wasm().getMomotoIdentity();
    },

    /** Run self-certification. */
    selfCertify(): string {
      return wasm().selfCertify();
    },
  },

  // ---------------------------------------------------------------------------
  // Events (delegates to Rust momoto-events)
  // ---------------------------------------------------------------------------

  events: {
    /** Create an event bus. */
    createBus(bufferSize = 100, maxAgeMs = 30000) {
      return wasm().MomotoEventBus.withConfig(bufferSize, BigInt(maxAgeMs));
    },

    /** Create a real-time event stream from a bus. */
    createStream(bus: InstanceType<WasmModule['MomotoEventBus']>) {
      return wasm().MomotoEventStream.fromBus(bus);
    },

    /** Create a batched event stream from a bus. */
    createBatchedStream(
      bus: InstanceType<WasmModule['MomotoEventBus']>,
      batchSize: number,
      timeoutMs: number,
    ) {
      return wasm().MomotoEventStream.fromBusBatched(bus, batchSize, BigInt(timeoutMs));
    },
  },

  // ---------------------------------------------------------------------------
  // Glass Materials (delegates to Rust momoto-materials)
  // ---------------------------------------------------------------------------

  glass: {
    clear() { return wasm().GlassMaterial.clear(); },
    regular() { return wasm().GlassMaterial.regular(); },
    thick() { return wasm().GlassMaterial.thick(); },
    frosted() { return wasm().GlassMaterial.frosted(); },
    builder() { return wasm().GlassMaterial.builder(); },
    liquid(variant: 'regular' | 'clear' = 'regular') {
      return new (wasm().LiquidGlass)(
        variant === 'clear' ? wasm().GlassVariant.Clear : wasm().GlassVariant.Regular,
      );
    },
  },

  // ---------------------------------------------------------------------------
  // CSS Rendering (delegates to Rust CssBackend)
  // ---------------------------------------------------------------------------

  css: {
    config: {
      default: () => new (wasm().CssRenderConfig)(),
      minimal: () => wasm().CssRenderConfig.minimal(),
      premium: () => wasm().CssRenderConfig.premium(),
      modal: () => wasm().CssRenderConfig.modal(),
      subtle: () => wasm().CssRenderConfig.subtle(),
      darkMode: () => wasm().CssRenderConfig.darkMode(),
    },
    /** Render a glass material to CSS. */
    render(material: InstanceType<WasmModule['GlassMaterial']>, context?: InstanceType<WasmModule['RenderContext']>) {
      const ctx = context ?? wasm().RenderContext.desktop();
      const backend = new (wasm().CssBackend)();
      const evalCtx = new (wasm().EvalMaterialContext)();
      const evaluated = material.evaluate(evalCtx);
      const css = backend.render(evaluated, ctx);
      backend.free();
      evalCtx.free();
      return css;
    },
  },

  // ---------------------------------------------------------------------------
  // Math utilities (delegates to Rust momoto-core::math)
  // ---------------------------------------------------------------------------

  math: {
    lerp: (a: number, b: number, t: number) => wasm().mathLerp(a, b, t),
    inverseLerp: (a: number, b: number, v: number) => wasm().mathInverseLerp(a, b, v),
    smoothstep: (t: number) => wasm().smoothstep(t),
    smootherstep: (t: number) => wasm().smootherstep(t),
    easeInOut: (t: number) => wasm().easeInOut(t),
    remap: (v: number, inMin: number, inMax: number, outMin: number, outMax: number) =>
      wasm().remap(v, inMin, inMax, outMin, outMax),
  },

  // ---------------------------------------------------------------------------
  // SIREN Neural Correction (delegates to Rust SIREN MLP)
  // Replaces: ventazo-web phase5-siren.ts
  // ---------------------------------------------------------------------------

  siren: {
    /** Compute SIREN neural correction for a foreground/background pair. */
    correct(
      bgL: number, bgC: number, bgH: number,
      fgL: number, fgC: number, fgH: number,
      apcaLc: number, wcagRatio: number, quality: number,
    ): SirenCorrectionResult {
      const result = wasm().computeSirenCorrection(
        bgL, bgC, bgH, fgL, fgC, fgH, apcaLc, wcagRatio, quality,
      );
      const correction = { deltaL: result.deltaL, deltaC: result.deltaC, deltaH: result.deltaH };
      result.free();
      return correction;
    },

    /** Apply SIREN correction to OKLCH values. Returns corrected [L, C, H]. */
    apply(l: number, c: number, h: number, deltaL: number, deltaC: number, deltaH: number): [number, number, number] {
      const arr = new Float64Array(wasm().applySirenCorrection(l, c, h, deltaL, deltaC, deltaH));
      return [arr[0]!, arr[1]!, arr[2]!];
    },

    /** Batch: Compute corrections for multiple pairs.
     *  Input: Float64Array of [bgL, bgC, bgH, fgL, fgC, fgH, apcaLc, wcagRatio, quality, ...] (9 per pair)
     *  Output: Float64Array of [deltaL, deltaC, deltaH, ...] (3 per pair) */
    correctBatch(inputs: Float64Array): Float64Array {
      return new Float64Array(wasm().computeSirenCorrectionBatch(inputs));
    },

    /** Get network metadata (architecture, params, seed). */
    metadata(): SirenMetadata {
      return wasm().sirenMetadata();
    },

    /** Export raw weights for inspection. */
    weights() {
      return wasm().sirenWeights();
    },
  },

  // ---------------------------------------------------------------------------
  // Refraction (delegates to Rust momoto-materials::refraction)
  // ---------------------------------------------------------------------------

  refraction: {
    /** Create refraction parameters. */
    params(ior: number, distortion: number, chromatic: number, edge: number) {
      return new (wasm().RefractionParams)(ior, distortion, chromatic, edge);
    },

    /** Presets */
    clear: () => wasm().RefractionParams.clear(),
    frosted: () => wasm().RefractionParams.frosted(),
    thick: () => wasm().RefractionParams.thick(),
    subtle: () => wasm().RefractionParams.subtle(),
    highIndex: () => wasm().RefractionParams.highIndex(),

    /** Calculate refraction at position. Returns Float64Array [offsetX, offsetY, hueShift, brightness]. */
    calculate(params: InstanceType<WasmModule['RefractionParams']>, x: number, y: number, incidentAngle = 0.0): Float64Array {
      return new Float64Array(wasm().calculateRefraction(params, x, y, incidentAngle));
    },

    /** Apply refraction to OKLCH. Returns Float64Array [l, c, h]. */
    applyToColor(
      params: InstanceType<WasmModule['RefractionParams']>,
      l: number, c: number, h: number,
      x: number, y: number,
      incidentAngle = 0.0,
    ): Float64Array {
      return new Float64Array(wasm().applyRefractionToColor(params, l, c, h, x, y, incidentAngle));
    },

    /** Generate distortion map. Returns Float64Array [offsetX, offsetY, hueShift, brightness, ...]. */
    distortionMap(params: InstanceType<WasmModule['RefractionParams']>, gridSize: number): Float64Array {
      return new Float64Array(wasm().generateDistortionMap(params, gridSize));
    },
  },

  // ---------------------------------------------------------------------------
  // Lighting (delegates to Rust momoto-materials::light_model)
  // ---------------------------------------------------------------------------

  lighting: {
    /** Create default lighting environment. */
    environment: () => new (wasm().LightingEnvironment)(),

    /** Calculate lighting for a surface.
     *  Returns { diffuse, specular, total, light_color } */
    calculate(
      normalX: number, normalY: number, normalZ: number,
      viewX: number, viewY: number, viewZ: number,
      env: InstanceType<WasmModule['LightingEnvironment']>,
      shininess: number,
    ) {
      return wasm().calculateLighting(normalX, normalY, normalZ, viewX, viewY, viewZ, env, shininess);
    },

    /** Derive physics-based gradient. */
    gradient(
      env: InstanceType<WasmModule['LightingEnvironment']>,
      surfaceCurvature: number,
      shininess: number,
      samples: number,
    ) {
      return wasm().deriveGradient(env, surfaceCurvature, shininess, samples);
    },

    /** Gradient to CSS with base color. */
    gradientCss(
      env: InstanceType<WasmModule['LightingEnvironment']>,
      surfaceCurvature: number,
      shininess: number,
      samples: number,
      baseL: number, baseC: number, baseH: number,
    ) {
      return wasm().gradientToCss(env, surfaceCurvature, shininess, samples, baseL, baseC, baseH);
    },

    /** Lighting context presets (for material evaluation). */
    context: {
      studio: () => wasm().LightingContext.studio(),
      outdoor: () => wasm().LightingContext.outdoor(),
      dramatic: () => wasm().LightingContext.dramatic(),
      soft: () => wasm().LightingContext.soft(),
      neutral: () => wasm().LightingContext.neutral(),
    },
  },

  // ---------------------------------------------------------------------------
  // Shadows (delegates to Rust momoto-materials::shadow_engine)
  // ---------------------------------------------------------------------------

  shadows: {
    /** Shadow parameter presets */
    standard: () => wasm().AmbientShadowParams.standard(),
    elevated: () => wasm().AmbientShadowParams.elevated(),
    subtle: () => wasm().AmbientShadowParams.subtle(),
    dramatic: () => wasm().AmbientShadowParams.dramatic(),

    /** Calculate ambient shadow CSS. Requires background OKLCH. */
    ambient(params: InstanceType<WasmModule['AmbientShadowParams']>, bgL: number, bgC: number, bgH: number, elevation: number): string {
      return wasm().calculateAmbientShadow(params, bgL, bgC, bgH, elevation);
    },

    /** Multi-scale ambient shadow CSS (3 layers). */
    multiScale(params: InstanceType<WasmModule['AmbientShadowParams']>, bgL: number, bgC: number, bgH: number, elevation: number): string {
      return wasm().calculateMultiScaleAmbient(params, bgL, bgC, bgH, elevation);
    },

    /** Interactive shadow that responds to UI state.
     *  state: 0=Rest, 1=Hover, 2=Active, 3=Focus */
    interactive(
      transition: InstanceType<WasmModule['ElevationTransition']>,
      state: number,
      bgL: number, bgC: number, bgH: number,
      glassDepth: number,
    ): string {
      return wasm().calculateInteractiveShadow(transition, state, bgL, bgC, bgH, glassDepth);
    },

    /** Elevation transition presets */
    transitions: {
      card: () => wasm().ElevationTransition.card(),
      fab: () => wasm().ElevationTransition.fab(),
      flat: () => wasm().ElevationTransition.flat(),
    },

    /** Material Design elevation value in dp. */
    elevationDp: (level: number) => wasm().elevationDp(level),

    /** Elevation surface tint opacity. */
    elevationTint: (level: number) => wasm().elevationTintOpacity(level),
  },

  // ---------------------------------------------------------------------------
  // PBR / BSDF (delegates to Rust momoto-materials::unified_bsdf)
  // ---------------------------------------------------------------------------

  pbr: {
    /** Create dielectric (glass) BSDF. */
    dielectric: (ior: number, roughness: number) => new (wasm().DielectricBSDF)(ior, roughness),
    glass: () => wasm().DielectricBSDF.glass(),
    water: () => wasm().DielectricBSDF.water(),
    diamond: () => wasm().DielectricBSDF.diamond(),

    /** Create conductor (metal) BSDF. */
    conductor: (n: number, k: number, roughness: number) => new (wasm().ConductorBSDF)(n, k, roughness),
    gold: () => wasm().ConductorBSDF.gold(),
    silver: () => wasm().ConductorBSDF.silver(),
    copper: () => wasm().ConductorBSDF.copper(),

    /** Create thin-film BSDF. */
    thinFilm: (thickness: number, filmIor: number, substrateIor: number) =>
      new (wasm().ThinFilmBSDF)(thickness, filmIor, substrateIor),
    soapBubble: (thickness = 350) => wasm().ThinFilmBSDF.soapBubble(thickness),
    oilSlick: (thickness = 500) => wasm().ThinFilmBSDF.oilOnWater(thickness),

    /** Create layered material stack. */
    layered: () => new (wasm().LayeredBSDF)(),

    /** Create Lambertian (diffuse) BSDF. albedo: 0-1. */
    lambertian: (albedo: number) => new (wasm().LambertianBSDF)(albedo),

    /** High-level PBR material from preset name. */
    material: (preset: string) => wasm().PBRMaterial.fromPreset(preset),

    /** PBR material builder for custom composition. */
    materialBuilder: () => wasm().PBRMaterial.builder(),

    /** Batch evaluate multiple materials. */
    evaluateBatch(iors: Float64Array, roughnesses: Float64Array, thicknesses: Float64Array, absorptions: Float64Array) {
      return wasm().evaluateMaterialBatch(iors, roughnesses, thicknesses, absorptions);
    },
  },

  // ---------------------------------------------------------------------------
  // Spectral Pipeline (delegates to Rust momoto-materials::spectral)
  // ---------------------------------------------------------------------------

  spectral: {
    /** Create a spectral pipeline for physically-based rendering. */
    pipeline: () => new (wasm().SpectralPipeline)(),

    /** Create a spectral signal. */
    signal: (wavelengths: Float64Array, intensities: Float64Array) =>
      new (wasm().SpectralSignal)(wavelengths, intensities),

    /** D65 illuminant spectral signal. */
    d65: () => wasm().SpectralSignal.d65Illuminant(),

    /** Uniform spectral signal. */
    uniform: (intensity: number) => wasm().SpectralSignal.uniformDefault(intensity),

    /** Default spectral sampling wavelengths (31 points, 380-780nm). */
    defaultSampling: () => wasm().getDefaultSpectralSampling(),

    /** High-resolution spectral sampling (81 points). */
    highResSampling: () => wasm().getHighResSpectralSampling(),

    /** Demonstrate spectral pipeline. */
    demo: () => wasm().demonstrateSpectralPipeline(),

    /** Flicker detection validator. */
    flickerValidator: () => new (wasm().FlickerValidator)(),
    flickerValidatorStrict: () => wasm().FlickerValidator.strict(),
    flickerValidatorRelaxed: () => wasm().FlickerValidator.relaxed(),
    flickerValidatorCustom: (stable: number, minor: number, warning: number) =>
      wasm().FlickerValidator.withThresholds(stable, minor, warning),
  },

  // ---------------------------------------------------------------------------
  // Temporal (delegates to Rust momoto-materials::temporal)
  // ---------------------------------------------------------------------------

  temporal: {
    /** Rate limiter for smooth transitions. */
    rateLimiter: (initial: number, maxRate: number, smooth = true) =>
      new (wasm().RateLimiter)(initial, maxRate, smooth),

    /** Exponential moving average. */
    ema: (alpha: number) => new (wasm().ExponentialMovingAverage)(alpha),

    /** Time-varying dielectric presets. */
    dryingPaint: () => wasm().TemporalDielectric.dryingPaint(),
    weatheringGlass: () => wasm().TemporalDielectric.weatheringGlass(),

    /** Time-varying thin film preset. */
    soapBubble: () => wasm().TemporalThinFilm.soapBubble(),

    /** Time-varying conductor preset. */
    heatedGold: () => wasm().TemporalConductor.heatedGold(),
  },

  // ---------------------------------------------------------------------------
  // Neural Constraints (delegates to Rust momoto-materials::neural_constraints)
  // ---------------------------------------------------------------------------

  constraints: {
    /** Create physics constraint validator. */
    validator: () => new (wasm().ConstraintValidator)(),

    /** Create validator with custom config. */
    validatorWithConfig: (energyTol: number, reciprocityTol: number, maxSpectralGrad: number, hardClamp: boolean) =>
      wasm().ConstraintValidator.withConfig(energyTol, reciprocityTol, maxSpectralGrad, hardClamp),
  },

  // ---------------------------------------------------------------------------
  // Delta-E Color Difference (delegates to Rust momoto-materials::perceptual_loss)
  // ---------------------------------------------------------------------------

  deltaE: {
    /** CIE delta-E76 (Euclidean in CIELAB). JND ~ 2.3. */
    e76: (l1: number, a1: number, b1: number, l2: number, a2: number, b2: number) =>
      wasm().deltaE76(l1, a1, b1, l2, a2, b2),

    /** CIE delta-E94 (improved weighting). */
    e94: (l1: number, a1: number, b1: number, l2: number, a2: number, b2: number) =>
      wasm().deltaE94(l1, a1, b1, l2, a2, b2),

    /** CIEDE2000 (state-of-the-art perceptual). */
    e2000: (l1: number, a1: number, b1: number, l2: number, a2: number, b2: number) =>
      wasm().deltaE2000(l1, a1, b1, l2, a2, b2),

    /** Batch CIEDE2000. Input: Float64Array [L1,a1,b1,L2,a2,b2,...]. */
    e2000Batch: (labPairs: Float64Array) =>
      new Float64Array(wasm().deltaE2000Batch(labPairs)),

    /** Convert sRGB to CIELAB. Returns [L, a, b]. */
    rgbToLab: (r: number, g: number, b: number): [number, number, number] => {
      const lab = new Float64Array(wasm().rgbToLab(r, g, b));
      return [lab[0]!, lab[1]!, lab[2]!];
    },

    /** Convert CIELAB to sRGB. Returns [r, g, b]. */
    labToRgb: (l: number, a: number, b: number): [number, number, number] => {
      const rgb = new Float64Array(wasm().labToRgb(l, a, b));
      return [rgb[0]!, rgb[1]!, rgb[2]!];
    },
  },

  // ---------------------------------------------------------------------------
  // Enhanced CSS (delegates to Rust momoto-materials::css_enhanced)
  // ---------------------------------------------------------------------------

  cssEnhanced: {
    /** Render enhanced CSS with all effects. Takes CssRenderConfig.
     *  material: EvaluatedMaterial from GlassMaterial.evaluate() */
    render(material: any, config?: InstanceType<WasmModule['CssRenderConfig']>): string {
      const cfg = config ?? new (wasm().CssRenderConfig)();
      return wasm().renderEnhancedCss(material, cfg);
    },

    /** Render premium CSS with animations.
     *  material: EvaluatedMaterial from GlassMaterial.evaluate() */
    renderPremium(material: any): string {
      return wasm().renderPremiumCss(material);
    },
  },

  // ---------------------------------------------------------------------------
  // Material Presets (delegates to Rust momoto-materials::enhanced_presets)
  // ---------------------------------------------------------------------------

  presets: {
    /** Get all enhanced glass presets. */
    glass: () => wasm().getEnhancedGlassPresets(),

    /** Get presets by quality tier ('low' | 'medium' | 'high' | 'ultra'). */
    byQuality: (tier: string) => wasm().getPresetsByQuality(tier),
  },
} as const;

// =============================================================================
// Type Definitions (for TS consumers that can't import from momoto-wasm)
// =============================================================================

export interface QualityScoreInput {
  overall: number;
  compliance: number;
  perceptual: number;
  appropriateness: number;
}

export interface SirenCorrectionResult {
  deltaL: number;
  deltaC: number;
  deltaH: number;
}

export interface SirenMetadata {
  architecture: number[];
  totalParams: number;
  omega0: number;
  seed: number;
  activations: string[];
  clampRanges: {
    deltaL: [number, number];
    deltaC: [number, number];
    deltaH: [number, number];
  };
  inputFeatures: string[];
}

// Re-export WASM types for convenience
export type { WasmModule };
