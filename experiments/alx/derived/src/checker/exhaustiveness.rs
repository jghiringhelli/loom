// ALX: derived from loom.loom §"check_exhaustiveness"
// Pattern match exhaustiveness: all match arms must be exhaustive over the matched type.
// ALX: body is raw text. We parse match blocks from the text and check variants.

use crate::ast::{Module, Item};
use crate::error::{LoomError, Span};
use std::collections::{HashMap, HashSet};

/// Lightweight enum variant registry.
struct EnumRegistry {
    variants_of: HashMap<String, Vec<String>>,
    enum_of_variant: HashMap<String, String>,
}

impl EnumRegistry {
    fn build(module: &Module) -> Self {
        let mut reg = EnumRegistry {
            variants_of: HashMap::new(),
            enum_of_variant: HashMap::new(),
        };
        // Pre-seed stdlib sum types
        for (enum_name, vs) in &[
            ("Option", vec!["Some", "None"]),
            ("Result", vec!["Ok", "Err"]),
        ] {
            let names: Vec<String> = vs.iter().map(|s| s.to_string()).collect();
            for name in &names {
                reg.enum_of_variant.insert(name.clone(), enum_name.to_string());
            }
            reg.variants_of.insert(enum_name.to_string(), names);
        }
        for item in &module.items {
            if let Item::Enum(ed) = item {
                let names: Vec<String> = ed.variants.iter().map(|v| v.name.clone()).collect();
                for name in &names {
                    reg.enum_of_variant.insert(name.clone(), ed.name.clone());
                }
                reg.variants_of.insert(ed.name.clone(), names);
            }
        }
        reg
    }
}

pub fn check_exhaustiveness(module: &Module) -> Result<(), Vec<LoomError>> {
    let registry = EnumRegistry::build(module);
    let mut errors = Vec::new();

    for item in &module.items {
        if let Item::Fn(fd) = item {
            // Join all body lines and parse match blocks from raw text
            let body_text = fd.body.join("\n");
            check_match_blocks(&body_text, &registry, fd.span, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Parse match blocks from raw body text and check exhaustiveness.
fn check_match_blocks(
    body: &str,
    registry: &EnumRegistry,
    span: Span,
    errors: &mut Vec<LoomError>,
) {
    // The parser reconstructs match blocks as:
    // "match x { Red => 1, Green => 2, Blue if (cond) => 3 }"
    let mut search_from = 0;
    while let Some(match_pos) = body[search_from..].find("match ") {
        let abs_pos = search_from + match_pos;
        let rest = &body[abs_pos..];
        // Find the opening brace
        if let Some(brace_start) = rest.find('{') {
            // Find matching closing brace
            let inner_start = brace_start + 1;
            let mut depth = 1i32;
            let mut brace_end = inner_start;
            for (i, c) in rest[inner_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            brace_end = inner_start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let inner = rest[inner_start..brace_end].trim();
            if !inner.is_empty() {
                let arms = split_match_arms(inner);
                check_arms(&arms, registry, span, errors);
            }
            search_from = abs_pos + brace_end + 1;
        } else {
            search_from = abs_pos + 6;
        }
    }
}

/// Split match arms by ", " at the top level (respecting nested braces/parens).
fn split_match_arms(inner: &str) -> Vec<(String, bool)> {
    let mut arms = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' | '[' | '{' => { depth += 1; current.push(chars[i]); }
            ')' | ']' | '}' => { depth -= 1; current.push(chars[i]); }
            ',' if depth == 0 => {
                let arm_text = current.trim().to_string();
                if !arm_text.is_empty() {
                    arms.push(parse_arm(&arm_text));
                }
                current.clear();
            }
            _ => current.push(chars[i]),
        }
        i += 1;
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        arms.push(parse_arm(&last));
    }
    arms
}

/// Parse a single arm "Pattern [if (guard)] => body" into (pattern, has_guard).
fn parse_arm(arm: &str) -> (String, bool) {
    // Format: "Pattern => body" or "Pattern if (guard) => body"
    let has_guard = arm.contains(" if ");
    let pattern = if let Some(if_pos) = arm.find(" if ") {
        arm[..if_pos].trim().to_string()
    } else if let Some(arrow_pos) = arm.find(" => ") {
        arm[..arrow_pos].trim().to_string()
    } else {
        arm.trim().to_string()
    };
    (pattern, has_guard)
}

fn check_arms(
    arms: &[(String, bool)],
    registry: &EnumRegistry,
    span: Span,
    errors: &mut Vec<LoomError>,
) {
    if arms.is_empty() { return; }

    // Determine the enum by scanning for known variant names
    let enum_name = arms.iter().find_map(|(pattern, _)| {
        let variant_name = pattern.split_whitespace().next().unwrap_or(pattern);
        let variant_name = variant_name.split('(').next().unwrap_or(variant_name);
        registry.enum_of_variant.get(variant_name).map(|s| s.as_str())
    });

    let enum_name = match enum_name {
        Some(n) => n,
        None => return, // No known enum variants found
    };

    let all_variants = match registry.variants_of.get(enum_name) {
        Some(v) => v,
        None => return,
    };

    // Guard-free wildcard or variable binding = total cover
    let has_total_cover = arms.iter().any(|(pattern, has_guard)| {
        !has_guard && (pattern == "_" || is_variable_pattern(pattern))
    });
    if has_total_cover {
        return;
    }

    // Collect variants covered by guard-free arms
    let covered: HashSet<&str> = arms
        .iter()
        .filter(|(_, has_guard)| !has_guard)
        .filter_map(|(pattern, _)| {
            let name = pattern.split_whitespace().next().unwrap_or(pattern);
            let name = name.split('(').next().unwrap_or(name);
            if registry.enum_of_variant.contains_key(name) {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let mut missing: Vec<String> = all_variants
        .iter()
        .filter(|v| !covered.contains(v.as_str()))
        .cloned()
        .collect();

    if !missing.is_empty() {
        missing.sort();
        errors.push(LoomError::NonExhaustiveMatch {
            missing,
            span,
        });
    }
}

/// Returns true if the pattern is a variable binding (lowercase identifier).
fn is_variable_pattern(pattern: &str) -> bool {
    let p = pattern.trim();
    if p.is_empty() || p == "_" { return false; }
    // A variable pattern starts with a lowercase letter and contains no spaces
    p.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) && !p.contains(' ')
}
