// proof.rs — emitted by: loom compile proof.loom
// Theory: Gradual Typing (Siek & Taha 2006)
// Dynamic type is an explicit escape hatch. Every use generates a checked cast.
// Static regions are fully verified by the Rust type system.

/// The Dynamic type: a value whose type is verified at runtime.
#[derive(Debug, Clone)]
pub enum Dynamic {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

impl Dynamic {
    pub fn from_int(n: i64) -> Self { Dynamic::Int(n) }
    pub fn from_str(s: &str) -> Self { Dynamic::Str(s.to_string()) }

    /// Checked cast to String. Gradual guarantee: never silently wrong.
    pub fn as_string(&self) -> Option<&str> {
        match self { Dynamic::Str(s) => Some(s.as_str()), _ => None }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self { Dynamic::Int(n) => Some(*n), _ => None }
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Dynamic::Int(n) => n.to_string(),
            Dynamic::Float(f) => f.to_string(),
            Dynamic::Str(s) => s.clone(),
            Dynamic::Bool(b) => b.to_string(),
        }
    }
}

// ── Static region: no Dynamic, fully verified ─────────────────────────────────

pub fn static_add(a: i64, b: i64) -> i64 { a + b }

// ── Gradual region: accepts Dynamic, verified before use ──────────────────────

pub fn gradual_process(value: Dynamic) -> String {
    value.to_display_string()
}

/// Checked cast: Dynamic -> concrete type with explicit failure handling
pub fn checked_length(value: Dynamic) -> i64 {
    match value.as_string() {
        Some(s) => s.len() as i64,
        None => 0, // on_failure: return 0
    }
}

// ── Mixed: static calling gradual ────────────────────────────────────────────

pub fn pipeline(x: i64) -> String {
    let doubled = static_add(x, x);
    gradual_process(Dynamic::from_int(doubled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_region_fully_type_safe() {
        assert_eq!(static_add(3, 4), 7);
        // Compiler verifies: cannot pass String to static_add
    }

    #[test]
    fn gradual_region_accepts_dynamic() {
        let result = gradual_process(Dynamic::from_int(42));
        assert_eq!(result, "42");
        let result2 = gradual_process(Dynamic::from_str("hello"));
        assert_eq!(result2, "hello");
    }

    #[test]
    fn checked_cast_succeeds_for_correct_type() {
        let value = Dynamic::from_str("hello");
        assert_eq!(checked_length(value), 5);
    }

    #[test]
    fn checked_cast_fails_gracefully_for_wrong_type() {
        let value = Dynamic::from_int(42);
        // Gradual guarantee: wrong type returns default, never panics
        assert_eq!(checked_length(value), 0);
    }

    #[test]
    fn pipeline_bridges_static_and_gradual() {
        let result = pipeline(5);
        assert_eq!(result, "10");
    }

    #[test]
    fn static_types_never_silently_bypassed() {
        // This test documents the key gradual typing property:
        // Dynamic values MUST go through checked casts before use as static types.
        let d = Dynamic::from_str("not a number");
        let as_int = d.as_int();
        assert!(as_int.is_none(), "gradual: wrong type must fail explicitly, not silently");
    }
}
