# ALX Phase 3 — Correction Log (Post-Correction-1)

S_realized pre-correction: 0 / 410 = 0.0000
S_realized post-correction: 139 / 410 = 0.3390

---

## Correction 1 (G1)
**Test(s) affected:** All test files
**Failure:** `cargo test` failed to compile any test — the derived `lib.rs` did not expose any public modules (`ast`, `checker`, `codegen`, `error`, `lexer`, `parser`, `project`, `lsp`). `LoomError` was not re-exported at the crate root.
**Root cause:** loom.loom had no "Public API" section specifying `lib.rs` module declarations.
**Spec improvement:** Added "Public API surface" section listing required `pub mod` declarations and `pub use error::LoomError` re-export.
**I∝(1-S)/S note:** This single gap caused S=0. Every test file failed to compile, yielding maximum correction cost.

---

## Correction 2 (G2)
**Test(s) affected:** All tests calling `Lexer::tokenize(src)`
**Failure:** `loom::lexer::Lexer` struct did not exist. Tests call `Lexer::tokenize(src)` (static method on a struct). The derived code only had a bare `lex()` function.
**Root cause:** Spec described `fn lex :: String -> ...` (pipeline function) without specifying the public struct wrapper.
**Spec improvement:** Added Lexer struct spec: `pub struct Lexer; impl Lexer { pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LoomError>> }`.
**I∝(1-S)/S note:** All parse/emit tests depend on `Lexer::tokenize`. High-multiplier gap.

---

## Correction 3 (G3)
**Test(s) affected:** All tests importing `RustEmitter`, `TypeScriptEmitter`, `OpenApiEmitter`, `JsonSchemaEmitter`, `SimulationEmitter`, `NeuroMLEmitter`
**Failure:** The derived code used bare functions (`emit_rust()`, etc.). Tests import struct types from specific submodule paths (e.g. `loom::codegen::rust::RustEmitter`).
**Root cause:** Spec used function names but real compiler uses zero-size structs with `new()+emit()`.
**Spec improvement:** Added emitter naming convention specifying each struct, its module path, and method signature. `NeuroMLEmitter::emit` is static (no `self`). Added `schema` module alias for `json_schema`.
**I∝(1-S)/S note:** Affected all codegen tests (majority of the suite).

---

## Correction 4 (G4)
**Test(s) affected:** `effect_test.rs`, `privacy_test.rs`, `safety_test.rs`, `interface_test.rs`, `infoflow_test.rs`, `invariant_test.rs`, `typestate_test.rs`, `units_test.rs`
**Failure:** Checker structs (`SafetyChecker`, `PrivacyChecker`, `EffectChecker`, `TypeChecker`, `InfoFlowChecker`, `TypestateChecker`, `UnitsChecker`) did not exist. Tests import them from `loom::checker::`.
**Root cause:** Spec used function names (`check_effects()`, etc.) without specifying struct wrappers.
**Spec improvement:** Added checker naming convention. `SafetyChecker::check` is a static method (no `new()`). Others use `new().check()` pattern returning `Result<(), Vec<LoomError>>`.
**I∝(1-S)/S note:** All checker tests were unreachable.

---

## Correction 5 (G5)
**Test(s) affected:** `being_test.rs`, `autopoiesis_test.rs`, `evolve_test.rs`, `selfmod_test.rs`, `neuroml_test.rs`
**Failure:** `BeingDef` had `epigenetic: Option<EpigeneticBlock>` etc. Tests construct structs with `epigenetic_blocks: Vec<EpigeneticBlock>` (plural, Vec, `_blocks` suffix). Parser constructed old field names.
**Root cause:** Spec used singular Option fields; real compiler uses Vec with `_blocks` suffix.
**Spec improvement:** Updated BeingDef, EcosystemDef fields to Vec with `_blocks` suffix. Updated EpigeneticBlock structure (`signal`+`modifies`+`reverts_when`). Updated MorphogenBlock (`signal`, `threshold: String`, `produces: Vec<String>`). Updated QuorumBlock (`threshold: String`).
**I∝(1-S)/S note:** Cascaded through parser, codegen, checker requiring 20+ targeted edits.

---

## Correction 6 (G6)
**Test(s) affected:** `safety_test.rs`, `privacy_test.rs`, and any test constructing `Annotation {}`
**Failure:** Derived `Annotation` struct had `span: Span` field. Tests construct `Annotation { key: "...", value: "..." }` with no `span`, causing compile errors.
**Root cause:** Spec said "key/value" but derived code added `span` without specification guidance.
**Spec improvement:** Clarified `Annotation` type has only `key: String` and `value: String` — no span field. Also noted field is `key` (not `name` as G6 originally stated — tests use `key`).
**I∝(1-S)/S note:** Blocked compilation of safety and privacy tests.

---

## Correction 7 (G7)
**Test(s) affected:** `project_test.rs`
**Failure:** `loom::project` module did not exist. Tests import `ProjectManifest` and `build_project`.
**Root cause:** Project module was entirely unspecified in loom.loom.
**Spec improvement:** Added project module spec with `ProjectManifest { name, modules, output }`, `from_str()` parser, and `build_project(&[&str], &str) -> Result<(), Vec<LoomError>>`.
**I∝(1-S)/S note:** project_test.rs failed to compile; tests started running once module existed.

---

## Correction 8 (G8 — unlisted, discovered during compilation)
**Test(s) affected:** `exhaustiveness_test.rs`, `di_test.rs`, `inference_test.rs`, `wasm_test.rs`, `lsp_test.rs`
**Failure:** Tests pattern-match on `LoomError::NonExhaustiveMatch { missing, .. }`, `LoomError::UndeclaredDependency`, etc. The derived `LoomError` was a plain struct.
**Root cause:** Spec said "holds message + span" without specifying it should be an enum with typed variants.
**Spec improvement:** Updated LoomError to be an enum with variants `LexError`, `ParseError`, `TypeError`, `UnificationError`, `NonExhaustiveMatch { missing: Vec<String> }`, `UndeclaredDependency`, `WasmUnsupported`, `General`. `NonExhaustiveMatch.missing` is `Vec<String>` (tests call `.sort()` on it).
**I∝(1-S)/S note:** Unlocked exhaustiveness/inference/di/wasm test files entirely.

---

## Correction 9 (G9 — unlisted, discovered during compilation)
**Test(s) affected:** All tests calling `Parser::new(&tokens)`
**Failure:** Derived `Parser::new` took `Vec<Token>`. Tests call `Parser::new(&tokens)` (passing `&Vec<Token>`), which requires the signature to accept `&[Token]`.
**Root cause:** Spec didn't specify Parser::new parameter type.
**Spec improvement:** Added G9 to Public API section: `Parser::new(tokens: &[Token]) -> Self`.
**I∝(1-S)/S note:** Would have blocked all parse+emit tests if not fixed alongside G2.

---

## Correction 10 (PlasticityBlock structure — unlisted)
**Test(s) affected:** `selfmod_test.rs`
**Failure:** `PlasticityBlock` had `rules: Vec<PlasticityRule>`. Tests expect flat `trigger/modifies/rule` fields. `PlasticityRule` was a struct; tests use `PlasticityRule::Hebbian` (enum pattern).
**Root cause:** Spec modeled plasticity as a list of named rules; real impl is one rule per block.
**Spec improvement:** `PlasticityBlock` now has `trigger: String, modifies: String, rule: PlasticityRule, span: Span`. `PlasticityRule` is an enum (renamed from `PlasticityStrategy`).
**I∝(1-S)/S note:** Enabled selfmod_test.rs to compile.
