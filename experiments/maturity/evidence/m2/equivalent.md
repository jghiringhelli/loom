# M2 equivalent / unkillable mutants (excluded from the score)

Equivalent (no observable behaviour change):

- smt_bridge.rs:183 delete match arm ">=" in translate_expr - fallthrough `_ => op_str` yields the identical ">=".
- smt_bridge.rs:184 delete match arm "<=" - same identity fallthrough.
- smt_bridge.rs:187 delete match arm ">" - same identity fallthrough.
- smt_bridge.rs:188 delete match arm "<" - same identity fallthrough.
- smt_bridge.rs:189 delete match arm "+" - same identity fallthrough.
- smt_bridge.rs:190 delete match arm "-" - same identity fallthrough.
- smt_bridge.rs:191 delete match arm "*" - same identity fallthrough.
- smt_bridge.rs:192 delete match arm "/" - same identity fallthrough.
- refinement.rs:82 delete match arm Expr::Literal(Bool(true)) in check_predicate_satisfiable - the arm only prints a warning to stderr; the `_` arm does nothing, and no error/return value differs.
- refinement.rs:163 delete match arm Item::Fn(_) in collect_known_types - falls to `_ => {}`, identical.
- refinement.rs:175 delete match arm Expr::Literal(_) in expr_contains_self - falls to `_ => false`, identical.

Environment-dependent (killable only where a `z3` binary is on PATH; not equivalent):

- smt_bridge.rs:252 replace == with != in discharge_smt (`stdout.trim() == "unsat"`). Test
  `discharge_proves_valid_triple_and_reports_unknown_for_invalid` asserts Proved/Unknown when z3 exists,
  but on this machine z3 is absent so the function returns Skipped on both paths. Counted as a survivor.
