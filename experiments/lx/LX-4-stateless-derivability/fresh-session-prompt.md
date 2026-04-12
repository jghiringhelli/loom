# LX-4 — Fresh Session Prompt

Copy this entire file as the **first message** to a brand-new LLM session.
Do not add any prior context. Do not explain the history of Loom.

---

You are helping me write a program in **Loom**, a specification-first language
that compiles to Rust.

I will give you the Loom language reference, then ask you to extend an existing
Loom file with a new feature. Your job is to produce valid Loom syntax that compiles
without errors.

## Loom Quick Reference (key syntax)

### Comments
```
-- This is a comment
```
(Note: `//` and `#` are NOT valid comments in Loom)

### Module
```loom
module ModuleName

-- declarations here

end
```

### Function with contracts
```loom
fn function_name :: InputType -> OutputType
  require: condition_expression
  ensure:  condition_expression
  body_expression
end
```

### Being (biological entity with behavioral contracts)
```loom
being EntityName
  describe: "description string"

  telos: "goal as a quoted string"
  end

  regulate:
    trigger: condition_expression
    action: action_fn_name
  end

  canalize:
    toward: target_identifier
    despite: [disturbance1, disturbance2]
    convergence_proof: proof_fn_name   -- optional
  end

  criticality:
    lower: 0.0
    upper: 1.0
    probe_fn: measure_fn_name          -- optional
  end

  epigenetic:
    signal: signal_name
    modifies: target_field
    reverts_when: condition_expression
    duration: N.unit
  end

  evolve:
    toward: telos
    search: | strategy_name
    constraint: "constraint description"
  end
end
```

### Lifecycle (top-level, NOT inside a being)
```loom
lifecycle EntityName :: State1 -> State2 -> State3
  checkpoint: CheckpointName
    requires: condition_fn_name
    on_fail: handler_fn_name
  end
end
```

### Niche construction (top-level)
```loom
niche_construction:
  modifies: target_field
  affects: [Module1, Module2]
  probe_fn: probe_fn_name              -- optional
end
```

### HGT adoption (top-level)
```loom
adopt: InterfaceName from ModuleName
```

### Types
```loom
type TypeName =
  field1: Type1,
  field2: Type2
end
```

### Require/ensure in functions
- `require:` checks preconditions (must be true on entry)
- `ensure:` checks postconditions (must be true on exit; use `result` for return value)
- Operators: `>`, `<`, `>=`, `<=`, `=`, `!=`, `and`, `or`, `not`

### Important constraints
- Non-ASCII characters cause lex errors — use only ASCII
- `lifecycle` must be top-level (not nested inside `being`)
- `telos:` inside a being must end with its own `end`
- String literals must not contain em-dashes or special Unicode

---

Ready? Here is my feature request:

