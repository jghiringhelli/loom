// M104: journal: — Episodic Memory Primitive for Beings
//
// Validates journal: block parsing and JournalChecker behavior.
// Tulving (1972) episodic vs semantic memory → Squire (1987) declarative/procedural
// distinction → GS Five Memory Types → Loom `journal:` (M104).

use loom::ast::*;
use loom::checker::JournalChecker;
use loom::compile;
use loom::lexer::Lexer;
use loom::parser::Parser;

fn parse_ok(src: &str) -> Module {
    let tokens = Lexer::tokenize(src).expect("lex failed");
    Parser::new(&tokens).parse_module().expect("parse failed")
}

// ─── Test 1: journal: with record: and keep: parses ──────────────────────────

#[test]
fn test_m104_journal_parses() {
    let src = r#"
module Agent
  being Chronicler
    telos: "remember decisions"
    end
    journal:
      record: every evolve_step
      record: every telos_progress
      keep: last 1000
    end
  end
end
"#;
    let module = parse_ok(src);
    assert_eq!(module.being_defs.len(), 1);
    let being = &module.being_defs[0];
    let journal = being.journal.as_ref().expect("journal block should be present");
    assert_eq!(journal.records.len(), 2);
    assert!(journal.records.contains(&JournalRecord::EvolveStep));
    assert!(journal.records.contains(&JournalRecord::TelosProgress));
    assert_eq!(journal.keep_last, Some(1000));
}

// ─── Test 2: keep: last 0 → error ────────────────────────────────────────────

#[test]
fn test_m104_zero_keep_is_error() {
    let src = r#"
module Agent
  being Broken
    telos: "do something"
    end
    journal:
      keep: last 0
    end
  end
end
"#;
    let result = compile(src);
    assert!(
        result.is_err(),
        "keep: last 0 must be a compile error"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|e| format!("{}", e).contains("zero-size ring buffer")),
        "should mention zero-size ring buffer: {:?}", errors
    );
}

// ─── Test 3: record: evolve_step without evolve: block → warning ──────────────

#[test]
fn test_m104_record_without_evolve_is_warning() {
    let src = r#"
module Agent
  being Watcher
    telos: "watch"
    end
    journal:
      record: every evolve_step
      keep: last 100
    end
  end
end
"#;
    let tokens = Lexer::tokenize(src).unwrap();
    let module = Parser::new(&tokens).parse_module().unwrap();
    let checker = JournalChecker::new();
    let diagnostics = checker.check(&module);
    assert!(
        diagnostics.iter().any(|e| format!("{}", e).contains("[warn]")),
        "should warn about evolve_step without evolve block: {:?}", diagnostics
    );
}

// ─── Test 4: emit: "path/file.log" parses ────────────────────────────────────

#[test]
fn test_m104_emit_path_parses() {
    let src = r#"
module Agent
  being Auditor
    telos: "audit decisions"
    end
    journal:
      record: every state_transition
      emit: "audit/agent.log"
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    let journal = being.journal.as_ref().expect("journal should be present");
    assert_eq!(journal.emit_path.as_deref(), Some("audit/agent.log"));
}

// ─── Test 5: all four record types parse correctly ───────────────────────────

#[test]
fn test_m104_all_record_types_parse() {
    let src = r#"
module Agent
  being FullRecorder
    telos: "record everything"
    end
    evolve:
      constraint: accuracy > 0.5
    end
    journal:
      record: every evolve_step
      record: every telos_progress
      record: every state_transition
      record: every regulation_trigger
      keep: last 500
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    let journal = being.journal.as_ref().expect("journal should be present");
    assert_eq!(journal.records.len(), 4);
    assert!(journal.records.contains(&JournalRecord::EvolveStep));
    assert!(journal.records.contains(&JournalRecord::TelosProgress));
    assert!(journal.records.contains(&JournalRecord::StateTransition));
    assert!(journal.records.contains(&JournalRecord::RegulationTrigger));
    assert_eq!(journal.keep_last, Some(500));
}

// ─── Test 6: being without journal: is valid ──────────────────────────────────

#[test]
fn test_m104_being_without_journal_is_valid() {
    let src = r#"
module Agent
  being Simple
    telos: "simple being"
    end
  end
end
"#;
    let module = parse_ok(src);
    let being = &module.being_defs[0];
    assert!(
        being.journal.is_none(),
        "journal: should be optional; absence should not cause errors"
    );
    // Checker should produce no hard errors
    let checker = JournalChecker::new();
    let errors: Vec<_> = checker
        .check(&module)
        .into_iter()
        .filter(|e| !format!("{}", e).contains("[warn]"))
        .collect();
    assert!(errors.is_empty(), "no hard errors for being without journal: {:?}", errors);
}
