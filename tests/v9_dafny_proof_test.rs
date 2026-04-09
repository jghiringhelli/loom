//! V9: Curry-Howard / Dependent Types — Dafny proof scaffold emission.
//!
//! Every `proof:` annotation on a function now emits a `{FN}_DAFNY_PROOF: &str`
//! const whose value is a ready-to-run Dafny method stub.
//!
//! Claim: dependent types DECLARED → EMITTED (Howard 1980 / Curry-Howard)
//! Tool:  dafny verify <file>.dfy (dotnet tool install --global dafny)
//! Gate:  const present in emitted Rust + correct Dafny structure in value

use loom::compile;

// ── helpers ──────────────────────────────────────────────────────────────────

fn emit(src: &str) -> String {
    compile(src).expect("compile failed")
}

// ── structural_recursion strategy ────────────────────────────────────────────

#[test]
fn structural_recursion_emits_dafny_const() {
    let src = r#"
module Test
fn factorial @pure :: Int -> Int
proof: structural_recursion
factorial(1)
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("FACTORIAL_DAFNY_PROOF"),
        "expected FACTORIAL_DAFNY_PROOF const"
    );
}

#[test]
fn structural_recursion_const_contains_decreases() {
    let src = r#"
module Test
fn gcd @pure :: Int -> Int
proof: structural_recursion
gcd(1)
end
end"#;
    let out = emit(src);
    // Dafny uses `decreases` to prove well-founded recursion.
    assert!(
        out.contains("decreases"),
        "expected `decreases` clause in Dafny scaffold"
    );
}

#[test]
fn structural_recursion_const_references_fn_name() {
    let src = r#"
module Test
fn fib @pure :: Int -> Int
proof: structural_recursion
fib(1)
end
end"#;
    let out = emit(src);
    // The scaffold method name must reference the original fn.
    assert!(
        out.contains("fib_structural"),
        "expected `fib_structural` method in Dafny scaffold"
    );
}

// ── totality strategy ─────────────────────────────────────────────────────────

#[test]
fn totality_emits_dafny_const() {
    let src = r#"
module Test
fn describe @pure :: Bool -> String
proof: totality
match true
| true -> "yes"
| false -> "no"
end
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("DESCRIBE_DAFNY_PROOF"),
        "expected DESCRIBE_DAFNY_PROOF const"
    );
}

#[test]
fn totality_const_contains_function_method() {
    let src = r#"
module Test
fn classify @pure :: Bool -> String
proof: totality
match true
| true -> "pos"
| false -> "neg"
end
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("function method"),
        "expected `function method` in Dafny totality scaffold"
    );
}

// ── induction strategy ────────────────────────────────────────────────────────

#[test]
fn induction_emits_dafny_lemma() {
    let src = r#"
module Test
fn sumN @pure :: Int -> Int
proof: induction
1
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("SUMN_DAFNY_PROOF"),
        "expected SUMN_DAFNY_PROOF const"
    );
    assert!(
        out.contains("lemma"),
        "expected `lemma` keyword in Dafny induction scaffold"
    );
}

#[test]
fn induction_const_contains_base_and_inductive_step() {
    let src = r#"
module Test
fn power @pure :: Int -> Int
proof: induction
1
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("Base case"),
        "expected base case comment in induction scaffold"
    );
    assert!(
        out.contains("Inductive"),
        "expected inductive step comment in induction scaffold"
    );
}

// ── contradiction strategy ────────────────────────────────────────────────────

#[test]
fn contradiction_emits_dafny_lemma() {
    let src = r#"
module Test
fn unique @pure :: Int -> Int
proof: contradiction
1
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("UNIQUE_DAFNY_PROOF"),
        "expected UNIQUE_DAFNY_PROOF const"
    );
    assert!(
        out.contains("contradiction") || out.contains("False") || out.contains("assume"),
        "expected contradiction language in Dafny scaffold"
    );
}

// ── unknown strategy gracefully handled ──────────────────────────────────────

#[test]
fn unknown_strategy_emits_generic_scaffold() {
    let src = r#"
module Test
fn compute @pure :: Int -> Int
proof: wellFounded
1
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("COMPUTE_DAFNY_PROOF"),
        "expected COMPUTE_DAFNY_PROOF const"
    );
    // The generic scaffold still emits something mentioning the strategy.
    assert!(
        out.contains("wellFounded"),
        "expected strategy name in generic scaffold"
    );
}

// ── multiple proof strategies ─────────────────────────────────────────────────

#[test]
fn multiple_proofs_emit_multiple_scaffolds() {
    let src = r#"
module Test
fn sorted @pure :: Int -> Int
proof: structural_recursion
proof: induction
sorted(1)
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("SORTED_DAFNY_PROOF"),
        "expected SORTED_DAFNY_PROOF const"
    );
    // Both strategies must appear in the scaffold body.
    assert!(
        out.contains("sorted_structural"),
        "expected structural method"
    );
    assert!(out.contains("sorted_induction"), "expected induction lemma");
}

// ── Curry-Howard audit comment ────────────────────────────────────────────────

#[test]
fn dafny_scaffold_includes_loom_audit_comment() {
    let src = r#"
module Test
fn reduce @pure :: Int -> Int
proof: structural_recursion
reduce(1)
end
end"#;
    let out = emit(src);
    assert!(
        out.contains("LOOM[V9:Dafny]"),
        "expected LOOM[V9:Dafny] audit comment"
    );
}

#[test]
fn dafny_scaffold_includes_install_instructions() {
    let src = r#"
module Test
fn solve @pure :: Int -> Int
proof: induction
1
end
end"#;
    let out = emit(src);
    // The scaffold must tell the developer how to run Dafny.
    assert!(
        out.contains("dafny verify") || out.contains("dafny"),
        "expected dafny usage instruction in scaffold"
    );
}
