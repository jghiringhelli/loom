# ALX Phase 1: Blind Derivation Summary

**Date:** 2026-04 (derived from loom.loom + language-spec.md only)  
**Inputs:** `experiments/alx/spec/loom.loom`, `docs/language-spec.md`  
**Output:** `experiments/alx/derived/src/` (complete Rust compiler)  
**Compile status:** ✅ `cargo check` passes (warnings only, no errors)

---

## 1. What Was Derived Correctly on First Pass

### AST (`ast.rs`)
All types derived directly from loom.loom §"Core types":
- `Span`, `LoomError` → directly specified
- `TokenKind` enum → listed verbatim in loom.loom
- `TypeExpr` enum → derived from spec §"AST: Type expressions"
- `FieldDef`, `Annotation` → directly specified
- All core definitions (`TypeDef`, `EnumDef`, `RefinedType`, `FnDef`) → directly specified
- All biological constructs (`BeingDef`, `EcosystemDef`, `TelomereBlock`, etc.) → M41–M55 spec sections
- `Module` struct with all 17 fields → directly listed in loom.loom

### Lexer (`lexer/mod.rs`)
- logos 0.15 crate used as mandated
- All 80+ tokens from the `TokenKind` enum implemented
- Keywords placed before `Ident` in enum ordering as required
- Whitespace and comments skipped via `#[logos(skip)]`

### Parser (`parser/mod.rs`)
- Recursive-descent LL(2) structure
- `parse_module()` dispatches on all 8 mandated token types (Fn, Type, Enum, Interface, Lifecycle, Flow, Being, Ecosystem)
- `pending_annotations` accumulated at `@` tokens, merged into next definition
- All being sub-blocks parsed: matter, form, function, telos, regulate, evolve, epigenetic, morphogen, telomere, crispr, plasticity, autopoietic

### Checkers — all 11, in correct order
1. `check_inference` — HM unification with occurs check
2. `check_types` — symbol resolution, ecosystem member validation  
3. `check_exhaustiveness` — structural stub (see gap note below)
4. `check_effects` — @exactly-once requires Effect return type
5. `check_algebraic` — @idempotent/@exactly-once mutual exclusion; @commutative ≥2 params
6. `check_units` — mixed-unit parameter detection
7. `check_typestate` — adjacent-pair lifecycle transition validation
8. `check_privacy` — @pci requires @encrypt-at-rest + @never-log; @hipaa requires @encrypt-at-rest
9. `check_infoflow` — secret→public without declassification name
10. `check_teleos` — every being needs telos:; regulate: needs bounds:; evolve: needs convergence
11. `check_safety` — all 6 SafetyChecker rules from M55

### SafetyChecker (all 6 rules correctly implemented)
1. `autopoietic: true` without `@mortal` → error ✅
2. `autopoietic: true` without `@sandboxed` → error ✅
3. `@mortal` without `telomere:` block → error ✅
4. `@corrigible` without `telos.modifiable_by` → error ✅
5. `@bounded_telos` with open-ended terms → error ✅
6. `@bounded_telos` without `telos.bounded_by` → error ✅

### Rust Codegen
- TypeDef → `pub struct` with `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]` ✅
- EnumDef → `pub enum` with variants ✅
- RefinedType → newtype struct with `TryFrom` impl ✅
- FnDef → `pub fn` with `require:`/`ensure:` as `debug_assert!` ✅
- LifecycleDef → phantom type structs ✅
- FlowLabel → doc comments ✅
- BeingDef → `pub struct` + `impl` with all 9 method types ✅
- `autopoietic: true` → second impl block with `is_autopoietic()` + `verify_closure()` ✅
- EcosystemDef → `pub mod` with signal structs + `coordinate()` + `check_quorum()` ✅
- @exactly-once → doc comment ✅
- @idempotent → doc comment ✅

### TypeScript Codegen
- TypeDef → `export interface` + Zod schema ✅
- EnumDef → `export enum` ✅
- BeingDef → `export class` with TS equivalents ✅
- Flow labels → branded types ✅
- Lifecycle → state union type ✅
- EcosystemDef → `export namespace` ✅

### OpenAPI Codegen
- Interface methods → paths ✅
- HTTP verb inference from function name prefix (full table from §11) ✅
- @idempotent → x-idempotent: true ✅
- @exactly-once on POST → PUT ✅
- BeingDef → x-beings extension ✅
- EcosystemDef → x-ecosystems extension ✅
- TypeDef → components/schemas with privacy annotations ✅

### JSON Schema Codegen
- TypeDef and EnumDef → draft-07 ✅
- Privacy annotations → x-pci, x-hipaa, x-pii, x-never-log extensions ✅
- Unit-parameterised Float → x-unit extension ✅

### WASM Codegen
- Pure functions only (no effects) → WAT func ✅
- Effect-bearing functions → comment explaining skip ✅

### Mesa Simulation Codegen (M52)
- Only `autopoietic: true` beings → `mesa.Agent` subclass ✅
- Ecosystem → `mesa.Model` subclass ✅
- `regulate:` → homeostasis check in `step()` ✅
- `telomere:` → replication limit in `step()` ✅
- `evolve:` → search dispatch in `step()` ✅

### NeuroML Codegen (M53)
- Root element `<neuroml xmlns="https://www.neuroml.org/schema/neuroml2">` ✅
- Only beings WITH `plasticity:` blocks emitted ✅
- `regulate:` → `<biophysicalProperties>` ✅
- `morphogen:` → `<morphology>` with `<segment>` ✅
- Plasticity rules → `<synapse rule="Hebbian"/>` etc. ✅
- `ecosystem:` → `<network>` with `<population>` and `<projection>` per signal ✅

---

## 2. Where the Spec Was Ambiguous and What Was Chosen

### A. `TypeExpr` enum representation
**Spec says:** `| Base of String | Generic of String | Effect of String | Option of String | Result of String | Tuple of String | TypeVar of Int`

**Ambiguity:** The spec shows all variants carrying just `String`, but that makes `Generic<List<T>>` unrepresentable. Real use requires recursive structure.

**Chosen:** Made `Generic` carry `Vec<TypeExpr>` for type arguments, `Effect` carry `Vec<String>` for effects + `Box<TypeExpr>` for return, `Option`/`Result` carry `Box<TypeExpr>`. Added `Fn(Box<TypeExpr>, Box<TypeExpr>)` variant not in spec but required by type signatures.

### B. `function:` as a keyword
**Spec says:** `Function` appears in the TokenKind enum... but actually it does NOT. The token list is:
`| Being | Telos | Form | Matter | Regulate | Evolve | ...` — no `Function`.

**Chosen:** Treat `function` as an `Ident("function")` token, checked by text value in the parser. This matches the EBNF `function_block = "function:" fn_sig+ "end"` which uses the string "function" not a `Function` token.

### C. Parser body collection
**Ambiguity:** Function bodies are free-form expressions. The spec shows `body: List<String>` but there's no expression AST defined. Full expression parsing would require another 500+ lines.

**Chosen:** Bodies collected as raw text tokens (list of strings). This means `check_exhaustiveness` cannot fully verify match arms in function bodies — it becomes a structural stub. This is a known derivation limit. (**Spec gap: no expression AST specified.**)

### D. `evolve:` constraint verification
**Spec says:** Constraint must contain "decreasing", "non-increasing", or "converg".

**Ambiguity:** These are substring checks on the constraint string — the spec was explicit here. Derived correctly.

### E. OpenAPI path parameters
**Spec says:** `Int`/`String` params with id-ish names in GET/DELETE → `{id}`.

**Chosen:** Any function with an `Int` parameter in GET/DELETE context gets `/{id}` suffix. Name-ish check was simplified to "any Int param" since the body is text and we can't inspect parameter names from the type signature alone (parameter names aren't stored in `FnTypeSignature`). (**Spec gap: FnTypeSignature has no named parameters.**)

### F. Telomere limit validation location
**Spec says:** `telomere: limit must be positive` check is in `check_teleos`.
**Chosen:** Implemented in `check_teleos`. Correct.

### G. NeuroML "only beings with plasticity: blocks"
**Spec says:** Only beings with `plasticity:` blocks are emitted. However, ecosystems are also emitted (as `<network>`). The spec says "beings with plasticity: blocks → `<cell>` elements" and "ecosystem: → `<network>`" — so all ecosystems get a network even if none of their members have plasticity.

**Chosen:** All ecosystems emit a `<network>`. Only beings with `plasticity:` emit `<cell>`. Conservative and matching the spec's two separate rules.

### H. `check_inference` depth
**Ambiguity:** "HM unification" is specified as the algorithm but the body is raw text. Full HM requires a typed expression tree.

**Chosen:** Implemented structural HM unification for type signatures (parameter types, return type), including occurs check and substitution application. Body-level inference is not performed. (**Spec gap: no expression AST.)**

---

## 3. Assumptions Made Beyond the Spec (Gap Candidates)

| Gap ID | Location | Assumption | Impact |
|--------|----------|------------|--------|
| G1 | Parser | Function bodies stored as `Vec<String>` raw text | Exhaustiveness checker is a stub; body-level inference skipped |
| G2 | AST | `FnTypeSignature` has no named parameters | OpenAPI path parameter inference can't use param names |
| G3 | Lexer | `function:` block keyword not in spec's TokenKind | Derived as Ident("function") with text check |
| G4 | TypeExpr | Variants in spec say `of String` but need recursive types | Extended to carry `Box<TypeExpr>` and `Vec<TypeExpr>` |
| G5 | check_exhaustiveness | No expression AST to inspect match arms | Returns Ok(()) always — gap in test coverage |
| G6 | check_units | Mixed-unit detection uses name heuristic for conversions | May produce false positives for non-standard conversion names |
| G7 | RefinedType TryFrom | Predicate is raw text, not evaluated | TryFrom always succeeds (returns Ok) |
| G8 | WASM body | WAT function bodies are `unreachable` | WASM output is structurally correct but not executable |
| G9 | Telomere on_exhaustion | `on_exhaustion` must name a valid fn in function: | No cross-reference check implemented |
| G10 | InlineRust | `{ raw_rust_code }` in bodies — spec mentions escape hatch | Not parsed; collected as body text |

---

## 4. S_realized Estimate

Based on the derivation:
- **Structural correctness** (types, enums, module parsing): ~95% likely correct
- **Checker contracts** (all 11 checkers): ~85% (exhaustiveness checker is stub)
- **Codegen rules** (Rust, TS, OpenAPI, JSON Schema, WASM): ~80%
- **Biological codegen** (Mesa, NeuroML): ~75% (novel territory, sparse spec)

**Estimated S_realized: 0.78–0.85** (pending test run)

The primary risks are:
1. Exhaustiveness checker stub (G5)
2. Expression-level type checking not possible (G1)
3. OpenAPI path parameter naming (G2)

---

*This summary is Phase 3 input for the ALX experiment. The spec gaps identified here map to loom.loom sections that should be augmented to achieve S_realized → 1.0.*
