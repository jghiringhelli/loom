//! M112 tests — CognitiveMemoryChecker: lightweight hippocampal layer.
//!
//! Validates that memory: type declarations are consistent with the being's
//! structural blocks. Inspired by Chronicle's five memory types but self-contained.

use loom::compile;

fn ok(src: &str) {
    let r = compile(src);
    assert!(r.is_ok(), "expected ok:\n{}", r.unwrap_err().iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
}

fn err_contains(src: &str, fragment: &str) {
    let r = compile(src);
    assert!(r.is_err(), "expected error containing '{}' but compiled ok", fragment);
    let msg = r.unwrap_err().iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
    assert!(msg.contains(fragment), "expected error containing '{}'\nGot:\n{}", fragment, msg);
}

// ── 1. Episodic without journal: → error ─────────────────────────────────────

#[test]
fn test_m112_episodic_without_journal_errors() {
    err_contains(r#"
module M
  being Agent
    telos: "agent"
    end
    memory:
      type: episodic
    end
  end
end
"#, "episodic");
}

// ── 2. Episodic WITH journal: → ok ────────────────────────────────────────────

#[test]
fn test_m112_episodic_with_journal_ok() {
    ok(r#"
module M
  being Agent
    telos: "agent"
    end
    journal:
      fields: event String
      retention: 100
    end
    memory:
      type: episodic
    end
  end
end
"#);
}

// ── 3. Procedural without migration: → error ─────────────────────────────────

#[test]
fn test_m112_procedural_without_migration_errors() {
    err_contains(r#"
module M
  being Agent
    telos: "agent"
    end
    memory:
      type: procedural
    end
  end
end
"#, "procedural");
}

// ── 4. Procedural WITH migration: → ok ───────────────────────────────────────

#[test]
fn test_m112_procedural_with_migration_ok() {
    ok(r#"
module M
  being Agent
    telos: "agent"
    end
    matter:
      value: Float
    end
    migration v1:
      from: value Float
      to:   value Double
      adapter: "fn v -> v as f64"
    end
    memory:
      type: procedural
    end
  end
end
"#);
}

// ── 5. Architectural without manifest: → error ────────────────────────────────

#[test]
fn test_m112_architectural_without_manifest_errors() {
    err_contains(r#"
module M
  being Agent
    telos: "agent"
    end
    memory:
      type: architectural
    end
  end
end
"#, "architectural");
}

// ── 6. Multiple types: episodic + procedural, both satisfied ─────────────────

#[test]
fn test_m112_multiple_types_all_satisfied_ok() {
    ok(r#"
module M
  being Agent
    telos: "agent"
    end
    journal:
      fields: event String
      retention: 50
    end
    matter:
      version: Int
    end
    migration v1:
      from: version Int
      to: version Int
      adapter: "fn v -> v + 1"
    end
    memory:
      type: episodic procedural
    end
  end
end
"#);
}

// ── 7. decay_rate out of range → error ───────────────────────────────────────

#[test]
fn test_m112_decay_rate_out_of_range_errors() {
    err_contains(r#"
module M
  being Agent
    telos: "agent"
    end
    journal:
      fields: event String
      retention: 50
    end
    memory:
      type: episodic
      decay_rate: 1.5
    end
  end
end
"#, "decay_rate");
}

// ── 8. decay_rate in range → ok ──────────────────────────────────────────────

#[test]
fn test_m112_valid_decay_rate_ok() {
    ok(r#"
module M
  being Agent
    telos: "agent"
    end
    journal:
      fields: event String
      retention: 50
    end
    memory:
      type: episodic
      decay_rate: 0.05
    end
  end
end
"#);
}

// ── 9. No memory: block → passes (memory: is optional) ───────────────────────

#[test]
fn test_m112_no_memory_block_is_ok() {
    ok(r#"
module M
  being Agent
    telos: "agent"
    end
  end
end
"#);
}
