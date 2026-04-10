//! Tests for M85: Randomness Quality Type System.
//!
//! Shannon (1948): entropy as unpredictability.
//! Blum-Blum-Shub (1986): CSPRNG formal definition.
//! NIST SP 800-90A (2012): deterministic random bit generator standards.

use loom::compile;
use loom::parse;

// ── M85 parsing tests ─────────────────────────────────────────────────────────

#[test]
fn test_m85_pseudo_random_alone_ok() {
    let src = r#"
module Simulation
  fn roll_dice @pseudo_random :: Unit -> Int
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "@pseudo_random alone should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_m85_crypto_random_alone_ok() {
    let src = r#"
module Security
  fn generate_nonce @crypto_random :: Unit -> String
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "@crypto_random alone should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_m85_true_random_alone_ok() {
    let src = r#"
module Entropy
  fn hardware_entropy @true_random :: Unit -> String
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_ok(),
        "@true_random alone should compile: {:?}",
        result.err()
    );
}

#[test]
fn test_m85_pseudo_random_requires_auth_rejected() {
    let src = r#"
module Auth
  fn generate_session_token @pseudo_random @requires_auth :: User -> String
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "@pseudo_random + @requires_auth should be rejected"
    );
    let err_msg = format!("{:?}", result.err());
    assert!(
        err_msg.contains("pseudo_random") || err_msg.contains("security context"),
        "error should mention pseudo_random or security context: {}",
        err_msg
    );
}

#[test]
fn test_m85_pseudo_random_pii_rejected() {
    let src = r#"
module Privacy
  fn anonymize_user @pseudo_random @pii :: User -> String
  end
end
"#;
    let result = compile(src);
    assert!(result.is_err(), "@pseudo_random + @pii should be rejected");
    let err_msg = format!("{:?}", result.err());
    assert!(
        err_msg.contains("pseudo_random") || err_msg.contains("security context"),
        "error should mention pseudo_random or security context: {}",
        err_msg
    );
}

#[test]
fn test_m85_seeded_parses() {
    let src = r#"
module Reproducible
  fn seeded_sample @seeded(42) :: Unit -> Float
  end
end
"#;
    let result = parse(src);
    assert!(
        result.is_ok(),
        "@seeded annotation should parse: {:?}",
        result.err()
    );
    let module = result.unwrap();
    if let loom::ast::Item::Fn(fd) = &module.items[0] {
        let seeded = fd.annotations.iter().find(|a| a.key == "seeded");
        assert!(seeded.is_some(), "seeded annotation should be present");
        assert_eq!(seeded.unwrap().value, "42");
    } else {
        panic!("expected a function item");
    }
}
