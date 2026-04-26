# momoto-ui — Perceptual Design System

Rust/WASM + TypeScript component library for color intelligence and UI primitives.

## Commands

```bash
# WASM (Rust)
npm run build:wasm                    # wasm-pack build → pkg/
cargo test                            # Rust unit tests
cargo clippy                          # Lint Rust code
wasm-pack build crates/momoto-wasm --target web --out-dir pkg  # Manual WASM build

# TypeScript
npm run build                         # tsup build
npm run test                          # vitest
npm run typecheck                     # tsc --noEmit
npm run verify:contract               # Verify TS↔WASM contract
```

## Architecture

- **Rust workspace**: `momoto/crates/momoto-wasm/` — 6 WASM modules + core
- **WASM output**: 244 exports (91 classes + 137 functions), ~2.0MB .wasm
- **Bridge**: `infrastructure/MomotoBridge.ts` — 20+ namespaces, 950 LOC
- **Domain**: `domain/` — PerceptualColor, ContrastAnalysis, MaterialPhase, etc.

## IMPORTANT Rules

- WASM objects need `.free()` after use (Color, OKLCH, ContrastResult)
- Use `ReturnType<typeof X.factoryMethod>` — NEVER `InstanceType<typeof X>` (private constructors)
- `pub type Elevation = u8` (NOT an enum) — use `ElevationPresets::LEVEL_X`
- Phase 3 uses CSS-native material approximations (WASM glass → monolithic CSS)
- Use `#[cfg(target_pointer_width)]` for wasm32 compatibility
- WASM needs browser (`--target web`); Node.js loads bridge but not WASM binary
