# Proof: Non-interference / Information Flow (Goguen & Meseguer, 1982)

**Theory:** A secure system is *non-interfering* if secret inputs cannot affect public outputs — i.e., an observer of public outputs learns nothing about secret inputs.  
**Claim:** Loom's `flow secret ::` tracks data sensitivity as a type parameter. A secret value flowing to a public output is a `LoomError::InformationFlowViolation` at compile time.  
**Key researchers:** Joseph Goguen & José Meseguer (1982).

## What is being proved

**Non-interference:** For all pairs of inputs that agree on public data, the public outputs are identical — regardless of the secret inputs. In Loom this is enforced structurally: `Sensitive<Secret, T>` and `Sensitive<Public, T>` are different types. You cannot pass one where the other is expected.

**Correct:** secret computation stays in secret context, or is explicitly declassified → compiles.  
**Violation:** `display(compute_bonus(salary))` → type error: `expected Public, found Secret`.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test public_data_flows_to_public_output ... ok
test secret_stays_in_secret_context ... ok
test declassify_is_the_only_path_from_secret_to_public ... ok
```

## Layman explanation

Like a government document classification system: a SECRET document cannot be included in a PUBLIC report without going through a formal declassification review. The Loom type system is the classification system — it physically prevents you from putting secret data into public outputs without an explicit, audited `declassify:` step.
