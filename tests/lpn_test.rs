/// LPN (Loom Protocol Notation) integration tests.
///
/// RED phase: these tests define the required behaviour before
/// any implementation exists.  They cover:
///   - Tier 1 parsing: FN, TYPE, ENUM, EMIT, CHECK, TEST, VERIFY
///   - Tier 2 parsing: IMPL … USING … EMIT … VERIFY
///   - Tier 3 parsing: ALX / SCALPER key=value params
///   - File-level execution: EMIT on a real .loom file
///   - File-level execution: CHECK on a valid and invalid .loom file
use loom::lpn::{
    CheckKind, EmitTarget, LpnError, LpnInstruction, LpnParser, LpnStatus, VerifyStep,
};

// ── Tier 1: FN ────────────────────────────────────────────────────────────────

#[test]
fn parse_fn_declaration() {
    let instr = LpnParser::parse_line("FN spread_bps :: Float -> Float -> Float").unwrap();
    match instr {
        LpnInstruction::Fn { name, sig } => {
            assert_eq!(name, "spread_bps");
            assert_eq!(sig, "Float -> Float -> Float");
        }
        other => panic!("expected Fn, got {:?}", other),
    }
}

// ── Tier 1: TYPE ──────────────────────────────────────────────────────────────

#[test]
fn parse_type_declaration() {
    let instr = LpnParser::parse_line("TYPE Tick = bid:Float ask:Float ts:Int").unwrap();
    match instr {
        LpnInstruction::Type { name, body } => {
            assert_eq!(name, "Tick");
            assert!(body.contains("bid:Float"));
        }
        other => panic!("expected Type, got {:?}", other),
    }
}

// ── Tier 1: EMIT ──────────────────────────────────────────────────────────────

#[test]
fn parse_emit_rust_bare() {
    let instr = LpnParser::parse_line("EMIT rust ScalpingAgent").unwrap();
    match instr {
        LpnInstruction::Emit {
            target: EmitTarget::Rust,
            module,
            from,
        } => {
            assert_eq!(module, "ScalpingAgent");
            assert!(from.is_none());
        }
        other => panic!("expected Emit, got {:?}", other),
    }
}

#[test]
fn parse_emit_rust_from_file() {
    let instr =
        LpnParser::parse_line("EMIT rust ScalpingAgent FROM experiments/scalper/scalper.loom")
            .unwrap();
    match instr {
        LpnInstruction::Emit {
            target: EmitTarget::Rust,
            module,
            from,
        } => {
            assert_eq!(module, "ScalpingAgent");
            assert_eq!(from.as_deref(), Some("experiments/scalper/scalper.loom"));
        }
        other => panic!("expected Emit, got {:?}", other),
    }
}

#[test]
fn parse_emit_typescript() {
    let instr =
        LpnParser::parse_line("EMIT ts PaymentAPI FROM examples/02-payment-api.loom").unwrap();
    match instr {
        LpnInstruction::Emit {
            target: EmitTarget::TypeScript,
            ..
        } => {}
        other => panic!("expected Emit TypeScript, got {:?}", other),
    }
}

#[test]
fn parse_emit_openapi() {
    let instr =
        LpnParser::parse_line("EMIT openapi PaymentAPI FROM examples/02-payment-api.loom").unwrap();
    match instr {
        LpnInstruction::Emit {
            target: EmitTarget::OpenApi,
            ..
        } => {}
        other => panic!("expected Emit OpenApi, got {:?}", other),
    }
}

// ── Tier 1: CHECK ─────────────────────────────────────────────────────────────

#[test]
fn parse_check_all() {
    let instr = LpnParser::parse_line("CHECK all examples/01-hello-contracts.loom").unwrap();
    match instr {
        LpnInstruction::Check {
            kind: CheckKind::All,
            file,
        } => {
            assert_eq!(file, "examples/01-hello-contracts.loom");
        }
        other => panic!("expected Check All, got {:?}", other),
    }
}

#[test]
fn parse_check_types() {
    let instr = LpnParser::parse_line("CHECK types examples/02-payment-api.loom").unwrap();
    match instr {
        LpnInstruction::Check {
            kind: CheckKind::Types,
            ..
        } => {}
        other => panic!("expected Check Types, got {:?}", other),
    }
}

#[test]
fn parse_check_contracts() {
    let instr = LpnParser::parse_line("CHECK contracts examples/02-payment-api.loom").unwrap();
    match instr {
        LpnInstruction::Check {
            kind: CheckKind::Contracts,
            ..
        } => {}
        other => panic!("expected Check Contracts, got {:?}", other),
    }
}

// ── Tier 1: TEST ──────────────────────────────────────────────────────────────

#[test]
fn parse_test_instruction() {
    let instr = LpnParser::parse_line("TEST spread_bps (100.0, 100.1) -> 10.0").unwrap();
    match instr {
        LpnInstruction::Test {
            name,
            args,
            expected,
        } => {
            assert_eq!(name, "spread_bps");
            assert_eq!(args, "100.0, 100.1");
            assert_eq!(expected, "10.0");
        }
        other => panic!("expected Test, got {:?}", other),
    }
}

// ── Tier 1: VERIFY ────────────────────────────────────────────────────────────

#[test]
fn parse_verify() {
    let instr = LpnParser::parse_line("VERIFY M84 experiments/scalper/scalper.loom").unwrap();
    match instr {
        LpnInstruction::Verify { claim, file } => {
            assert_eq!(claim, "M84");
            assert_eq!(file, "experiments/scalper/scalper.loom");
        }
        other => panic!("expected Verify, got {:?}", other),
    }
}

// ── Tier 1: comments and blank lines ─────────────────────────────────────────

#[test]
fn parse_comment_returns_none() {
    assert!(LpnParser::parse_line("# this is a comment").is_none());
}

#[test]
fn parse_blank_returns_none() {
    assert!(LpnParser::parse_line("   ").is_none());
}

// ── Tier 2: IMPL ──────────────────────────────────────────────────────────────

#[test]
fn parse_impl_with_milestones_and_verify() {
    let instr = LpnParser::parse_line(
        "IMPL ScalpingAgent USING [M41,M55,M84-M89] EMIT rust VERIFY compile",
    )
    .unwrap();
    match instr {
        LpnInstruction::Impl {
            target,
            milestones,
            emit: EmitTarget::Rust,
            verify,
        } => {
            assert_eq!(target, "ScalpingAgent");
            // M84-M89 expands to 6 milestones; M41 and M55 are singles → 8 total
            let expanded: Vec<u32> = milestones.iter().flat_map(|m| m.expand()).collect();
            assert_eq!(expanded.len(), 8);
            assert!(verify.contains(&VerifyStep::Compile));
        }
        other => panic!("expected Impl, got {:?}", other),
    }
}

#[test]
fn parse_impl_multi_verify() {
    let instr =
        LpnParser::parse_line("IMPL PaymentAPI USING [M19,M20,M21] EMIT rust VERIFY compile+types")
            .unwrap();
    match instr {
        LpnInstruction::Impl { verify, .. } => {
            assert!(verify.contains(&VerifyStep::Compile));
            assert!(verify.contains(&VerifyStep::Types));
        }
        other => panic!("expected Impl, got {:?}", other),
    }
}

// ── Tier 3: ALX ───────────────────────────────────────────────────────────────

#[test]
fn parse_alx_params() {
    let instr = LpnParser::parse_line(
        "ALX n=7 domain=biotech coverage>=0.95 emit=rust verify=compile+run evidence=store",
    )
    .unwrap();
    match instr {
        LpnInstruction::Alx(params) => {
            assert_eq!(params.n, Some(7));
            assert_eq!(params.domain.as_deref(), Some("biotech"));
            assert_eq!(params.min_coverage, Some(0.95));
            assert_eq!(params.emit, EmitTarget::Rust);
            assert!(params.verify.contains(&VerifyStep::Compile));
            assert!(params.verify.contains(&VerifyStep::Run));
            assert!(params.evidence);
        }
        other => panic!("expected Alx, got {:?}", other),
    }
}

#[test]
fn parse_scalper_experiment() {
    let instr = LpnParser::parse_line(
        "SCALPER ticks=10000 ou_theta=2.0 ou_sigma=0.15 emit=rust run=backtest",
    )
    .unwrap();
    match instr {
        LpnInstruction::Experiment { name, params } => {
            assert_eq!(name, "SCALPER");
            assert!(params.iter().any(|(k, v)| k == "ticks" && v == "10000"));
            assert!(params.iter().any(|(k, _)| k == "ou_theta"));
        }
        other => panic!("expected Experiment, got {:?}", other),
    }
}

// ── File parsing ──────────────────────────────────────────────────────────────

#[test]
fn parse_file_skips_comments_and_blanks() {
    let src = "# setup\n\nEMIT rust Foo\nCHECK all foo.loom\n# done\n";
    let instrs = LpnParser::parse_str(src);
    assert_eq!(instrs.len(), 2);
}

// ── Execution: CHECK on a real file ──────────────────────────────────────────

#[test]
fn execute_check_on_valid_hello_contracts() {
    use loom::lpn::LpnExecutor;
    use std::path::PathBuf;

    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let executor = LpnExecutor::new(base.clone());

    let result = executor.execute(&LpnInstruction::Check {
        kind: CheckKind::All,
        file: "examples/01-hello-contracts.loom".into(),
    });

    assert!(
        matches!(result.status, LpnStatus::Ok),
        "expected Ok, got {:?}: {}",
        result.status,
        result.output.unwrap_or_default()
    );
}

// ── Error: unknown opcode ─────────────────────────────────────────────────────

#[test]
fn parse_unknown_opcode_returns_error() {
    let result = LpnParser::try_parse_line("FOOBAR something");
    assert!(matches!(result, Err(LpnError::Parse { .. })));
}
