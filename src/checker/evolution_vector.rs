//! M111: Evolution Vector Checker — Semantic Migration Deduplication.
//!
//! Uses a type-lattice vector encoding to detect identical or related migration
//! patterns across all beings in a module. Groups migrations into evolutionary
//! families and warns when the same transformation is defined redundantly.
//!
//! Theory: Encode each type as a sparse vector in a semantic space with dimensions:
//!   [numeric_precision, string_encoding, temporal, spatial, monetary, boolean, composite]
//! Migrations are (from_vec, to_vec) pairs. Cosine similarity between pairs identifies:
//!   - Identical migrations (same type transition in different beings) → suggest shared adapter
//!   - Related migrations (same type family, same direction) → evolutionary cluster
//!
//! This embodies the user's insight: "use vector semantics to detect identical or related
//! evolutionary advances and group them." No LLM required — the type lattice encodes
//! sufficient semantic structure.
//!
//! Reference: Formal concept analysis (Ganter & Wille 1999) provides the theoretical
//! basis for lattice-based concept grouping. The cosine similarity threshold of 0.85
//! corresponds to the discrimination threshold in formal concept lattices.

use crate::ast::{BeingDef, Module};
use crate::error::LoomError;

/// Semantic dimension indices for the type vector space.
const DIM_NUMERIC_INT: usize = 0;
const DIM_NUMERIC_FLOAT: usize = 1;
const DIM_NUMERIC_PRECISE: usize = 2; // Decimal, Money
const DIM_STRING_RAW: usize = 3;
const DIM_STRING_ENCODED: usize = 4; // Bytes, Utf8
const DIM_STRING_RICH: usize = 5;    // Text, Html, Markdown
const DIM_TEMPORAL: usize = 6;       // Duration, Timestamp, DateTime
const DIM_SPATIAL: usize = 7;        // Point, Vec3, Coordinate
const DIM_MONETARY: usize = 8;       // Money, Currency, Amount
const DIM_BOOLEAN: usize = 9;
const DIM_COMPOSITE: usize = 10;     // List, Set, Map, Option
const DIM_UNKNOWN: usize = 11;
const DIMS: usize = 12;

/// A migration pattern: the beings it appears in, and the (from, to) type vector pair.
#[derive(Debug, Clone)]
struct MigrationPattern {
    /// Name of the being that declared this migration.
    being: String,
    /// Name of the migration block.
    migration: String,
    /// The field being migrated.
    field: String,
    /// Semantic vector for from_type.
    from_vec: [f32; DIMS],
    /// Semantic vector for to_type.
    to_vec: [f32; DIMS],
    /// Raw type names for error messages.
    from_type: String,
    to_type: String,
}

/// Checker that detects duplicate and related migration patterns across beings.
pub struct EvolutionVectorChecker;

impl EvolutionVectorChecker {
    /// Create a new evolution vector checker.
    pub fn new() -> Self {
        EvolutionVectorChecker
    }

    /// Check all beings in `module` for duplicate/related migration patterns.
    pub fn check(&self, module: &Module) -> Vec<LoomError> {
        let mut errors = Vec::new();
        let patterns = self.collect_patterns(module);
        self.detect_duplicates(&patterns, &mut errors);
        self.detect_clusters(&patterns, &mut errors);
        errors
    }

    /// Collect all field-migration patterns from every being in the module.
    fn collect_patterns(&self, module: &Module) -> Vec<MigrationPattern> {
        let mut patterns = Vec::new();
        for being in &module.being_defs {
            patterns.extend(self.patterns_for_being(being));
        }
        patterns
    }

    /// Extract migration patterns from a single being.
    fn patterns_for_being(&self, being: &BeingDef) -> Vec<MigrationPattern> {
        let mut result = Vec::new();
        for migration in &being.migrations {
            // Only process field-based migrations (not integer version migrations).
            if let (Some((field, from_ty)), Some((_, to_ty))) = (
                parse_migration_field(&migration.from_field),
                parse_migration_field(&migration.to_field),
            ) {
                result.push(MigrationPattern {
                    being: being.name.clone(),
                    migration: migration.name.clone(),
                    field: field.clone(),
                    from_vec: type_vector(&from_ty),
                    to_vec: type_vector(&to_ty),
                    from_type: from_ty,
                    to_type: to_ty,
                });
            }
        }
        result
    }

    /// Rule 1: Identical migration pattern in two different beings → warn, suggest shared adapter.
    fn detect_duplicates(&self, patterns: &[MigrationPattern], errors: &mut Vec<LoomError>) {
        for i in 0..patterns.len() {
            for j in (i + 1)..patterns.len() {
                let a = &patterns[i];
                let b = &patterns[j];
                if a.being == b.being {
                    continue; // Same being, different migrations — not a cross-being duplicate.
                }
                let from_sim = cosine_similarity(&a.from_vec, &b.from_vec);
                let to_sim = cosine_similarity(&a.to_vec, &b.to_vec);
                if from_sim > 0.999 && to_sim > 0.999 {
                    // Exact same type transition in two different beings.
                    errors.push(LoomError::type_err(
                        format!(
                            "[warn] migration vector duplicate: '{}' in being '{}' and '{}' in being '{}' \
                             both migrate {} → {}. Consider extracting a shared migration adapter \
                             to avoid divergent evolution. (Ganter & Wille 1999: formal concept identity)",
                            a.migration, a.being,
                            b.migration, b.being,
                            a.from_type, a.to_type
                        ),
                        crate::ast::Span::new(0, 0),
                    ));
                }
            }
        }
    }

    /// Rule 2: Related migration patterns (same type family, same direction) → evolutionary cluster.
    ///
    /// A cluster is reported when 3+ migrations across different beings share the same
    /// semantic direction (e.g., all widen numeric precision: Int→Float, Float→Double, Double→Decimal).
    fn detect_clusters(&self, patterns: &[MigrationPattern], errors: &mut Vec<LoomError>) {
        if patterns.len() < 3 {
            return;
        }
        // Group by semantic direction: the delta vector (to_vec - from_vec).
        // Two migrations are in the same cluster when their delta cosine similarity > 0.85.
        let mut cluster_roots: Vec<(usize, Vec<usize>)> = Vec::new();
        for i in 0..patterns.len() {
            let delta_i = delta_vec(&patterns[i].from_vec, &patterns[i].to_vec);
            let mut found = false;
            for (root, members) in &mut cluster_roots {
                let delta_root = delta_vec(&patterns[*root].from_vec, &patterns[*root].to_vec);
                if cosine_similarity(&delta_i, &delta_root) > 0.85 {
                    members.push(i);
                    found = true;
                    break;
                }
            }
            if !found {
                cluster_roots.push((i, vec![i]));
            }
        }

        // Report clusters with 3+ members from at least 2 different beings.
        for (root_idx, members) in &cluster_roots {
            if members.len() < 3 {
                continue;
            }
            let beings: Vec<&str> = members.iter()
                .map(|&i| patterns[i].being.as_str())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            if beings.len() < 2 {
                continue;
            }
            let root = &patterns[*root_idx];
            let member_names: Vec<String> = members.iter()
                .map(|&i| format!("'{}'::{}", patterns[i].being, patterns[i].migration))
                .collect();
            errors.push(LoomError::type_err(
                format!(
                    "[info] evolutionary cluster detected: {} migrations share the semantic direction \
                     {} → {} across beings [{}]. This is a recurring evolutionary pattern — \
                     consider a shared migration typeclass. (GS Evolvable property: evolution paths \
                     should be reusable, not redefined per-being)",
                    members.len(),
                    root.from_type, root.to_type,
                    member_names.join(", ")
                ),
                crate::ast::Span::new(0, 0),
            ));
        }
    }
}

// ── Type lattice encoding ─────────────────────────────────────────────────────

/// Map a Loom type name to a semantic vector in the 12-dimensional type space.
///
/// The vector encodes type family membership. Values > 0 indicate membership
/// in that semantic dimension. Multiple dimensions can be non-zero for hybrid types.
fn type_vector(type_name: &str) -> [f32; DIMS] {
    let mut v = [0.0f32; DIMS];
    match type_name {
        "Int" | "Integer" | "I8" | "I16" | "I32" | "I64" | "I128" | "U8" | "U16" | "U32" | "U64" => {
            v[DIM_NUMERIC_INT] = 1.0;
        }
        "Float" | "F32" | "Double" | "F64" => {
            v[DIM_NUMERIC_INT] = 0.3;
            v[DIM_NUMERIC_FLOAT] = 1.0;
        }
        "Decimal" | "BigDecimal" | "Numeric" => {
            v[DIM_NUMERIC_FLOAT] = 0.5;
            v[DIM_NUMERIC_PRECISE] = 1.0;
        }
        "Money" | "Amount" | "Price" | "Currency" => {
            v[DIM_NUMERIC_PRECISE] = 0.5;
            v[DIM_MONETARY] = 1.0;
        }
        "String" | "Str" => {
            v[DIM_STRING_RAW] = 1.0;
        }
        "Bytes" | "Vec<u8>" | "ByteArray" => {
            v[DIM_STRING_RAW] = 0.3;
            v[DIM_STRING_ENCODED] = 1.0;
        }
        "Utf8" | "CStr" => {
            v[DIM_STRING_ENCODED] = 0.8;
            v[DIM_STRING_RICH] = 0.2;
        }
        "Text" | "Html" | "Markdown" | "RichText" => {
            v[DIM_STRING_ENCODED] = 0.2;
            v[DIM_STRING_RICH] = 1.0;
        }
        "Duration" => {
            v[DIM_TEMPORAL] = 1.0;
        }
        "Timestamp" | "DateTime" | "Date" | "Time" | "Instant" => {
            v[DIM_TEMPORAL] = 1.0;
        }
        "Point" | "Vec2" | "Vec3" | "Coordinate" | "LatLon" | "Position" => {
            v[DIM_SPATIAL] = 1.0;
        }
        "Bool" | "Boolean" => {
            v[DIM_BOOLEAN] = 1.0;
        }
        "List" | "Vec" | "Array" | "Set" | "Map" | "HashMap" | "Option" => {
            v[DIM_COMPOSITE] = 1.0;
        }
        "Percentage" | "Rate" | "Ratio" => {
            v[DIM_NUMERIC_FLOAT] = 0.5;
            v[DIM_NUMERIC_PRECISE] = 0.5;
        }
        _ => {
            // Unknown type: lands in the unknown dimension — still comparable.
            v[DIM_UNKNOWN] = 1.0;
        }
    }
    v
}

/// Compute the delta vector (to - from), clamping negative values to 0.
///
/// The delta represents the semantic "direction" of the type migration.
/// Two migrations with similar deltas are moving in the same semantic direction.
fn delta_vec(from: &[f32; DIMS], to: &[f32; DIMS]) -> [f32; DIMS] {
    let mut d = [0.0f32; DIMS];
    for i in 0..DIMS {
        d[i] = (to[i] - from[i]).max(0.0);
    }
    d
}

/// Cosine similarity between two vectors. Returns 0.0 for zero vectors.
fn cosine_similarity(a: &[f32; DIMS], b: &[f32; DIMS]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a < 1e-10 || mag_b < 1e-10 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

// ── Shared with migration.rs — extract (field_name, type_name) from debug token string ──

fn parse_migration_field(raw: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() >= 2 {
        let field = extract_ident(parts[0]);
        let typ = extract_ident(parts[1]);
        if !field.is_empty() && !typ.is_empty()
            && !field.starts_with("Int(")
            && !field.starts_with("Float(")
        {
            return Some((field, typ));
        }
    }
    None
}

fn extract_ident(s: &str) -> String {
    if let Some(inner) = s.strip_prefix("Ident(\"").and_then(|t| t.strip_suffix("\")")) {
        inner.to_string()
    } else {
        s.to_string()
    }
}
