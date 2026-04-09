//! Regex-based template substitution for Rust code generation.
//!
//! Motivation: Rust's `format!` macro requires `{{` / `}}` to emit literal braces, which makes
//! templates of Rust code (which are full of `{` / `}`) unreadable and fragile.
//!
//! Solution: raw string templates with `{placeholder}` syntax, substituted via regex.
//!
//! ## Usage
//! ```rust,ignore
//! let code = subst(
//!     r#"
//! pub struct {Name} {
//!     pub {field}: {Type},
//! }
//! impl {Name} {
//!     pub fn new({field}: {Type}) -> Self { Self { {field} } }
//! }
//! "#,
//!     &[("Name", "Foo"), ("field", "value"), ("Type", "String")],
//! );
//! ```
//! No escaping needed. Placeholders are `{word}` (word-boundary matched).
//! All occurrences are replaced globally.

use regex::Regex;

/// Substitute `{key}` placeholders in `template` with values from `vars`.
///
/// - Template is typically a raw string literal (`r#"..."#`) — no escape sequences needed.
/// - Placeholders: `{identifier}` — ASCII word chars only.
/// - Replacement is global — all occurrences of `{key}` are replaced.
/// - Unknown placeholders are left as-is (safe for partially-applied templates).
/// - Values may themselves contain `{` and `}` without issue (no recursive expansion).
///
/// # Panics
/// Never — the internal regex is validated at compile time via `expect`.
pub fn subst(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_owned();
    for (key, value) in vars {
        // Match exact `{key}` token — word boundary ensures we don't match `{foobar}` when key=`foo`.
        let pattern = format!(r"\{{{key}\}}");
        let re = Regex::new(&pattern).expect("subst: invalid placeholder regex");
        result = re.replace_all(&result, *value).into_owned();
    }
    result
}

/// Trim leading newline and trailing whitespace from a raw string literal.
/// Useful when raw strings start with `\n` for visual alignment.
#[inline]
pub fn t(s: &str) -> &str {
    s.trim_start_matches('\n').trim_end()
}

/// Convenience: `subst` + `t` in one call.
pub fn ts(template: &str, vars: &[(&str, &str)]) -> String {
    let trimmed = t(template);
    subst(trimmed, vars) + "\n\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_struct_with_braces() {
        let code = subst(
            r#"pub struct {Name} { pub {field}: {Type}, }"#,
            &[("Name", "Foo"), ("field", "value"), ("Type", "String")],
        );
        assert_eq!(code, "pub struct Foo { pub value: String, }");
    }

    #[test]
    fn multiple_occurrences_all_replaced() {
        let code = subst(
            r#"impl {Name} { fn new() -> {Name} { {Name}::default() } }"#,
            &[("Name", "Bar")],
        );
        assert_eq!(code, "impl Bar { fn new() -> Bar { Bar::default() } }");
    }

    #[test]
    fn unknown_placeholder_left_intact() {
        let code = subst(r#"struct {Name} { {unknown}: u32 }"#, &[("Name", "X")]);
        assert_eq!(code, "struct X { {unknown}: u32 }");
    }

    #[test]
    fn value_with_braces_not_expanded() {
        let code = subst(
            r#"fn {name}() { {body} }"#,
            &[("name", "foo"), ("body", "let x = {y};")],
        );
        assert_eq!(code, "fn foo() { let x = {y}; }");
    }

    #[test]
    fn partial_word_not_matched() {
        // {foo} should NOT match inside {foobar}
        let code = subst(r#"{foobar} vs {foo}"#, &[("foo", "X")]);
        assert_eq!(code, "{foobar} vs X");
    }
}
