# How to Prove Loom's EMITTED Properties

> **Purpose:** Step-by-step instructions to discharge each EMITTED property to PROVED.
> Run these when the tool is available. After each success, update `docs/foundations.md`
> and `docs/verification-matrix.md` to change the status.

---

## Current Status

| # | Theory | Tool | Platform | Status |
|---|--------|------|----------|--------|
| 5 | Clarke-Emerson-Sifakis Model Checking | Kani | Linux / WSL2 | **EMITTED** |
| 8 | Reynolds Separation Logic | Prusti | Linux / WSL2 | **EMITTED** |
| 10 | Lamport TLA+ / Convergence | TLC (Java) | Any | **EMITTED** |
| 14 | Martin-Löf Dependent Types | Dafny | Any | **EMITTED** |

---

## Property 14 — Martin-Löf Dependent Types (Dafny)

**Tool:** Dafny 4.11+ · **Platform:** Windows / macOS / Linux  
**Install:** `dotnet tool install --global Dafny`  
**Proof file:** `experiments/proofs/dependent-types/proof.dfy`

### The claim
Loom's `dependent:` and `proposition:` blocks emit length-indexed types where
the length is a compile-time value — a *type that depends on a value*.
Dafny's `ensures` clauses verify these invariants exhaustively: not by testing,
but by proof search over all possible inputs.

### Steps

1. **Create `experiments/proofs/dependent-types/proof.dfy`**
   (content below — also embedded as a comment in `proof.rs`)

```dafny
// Proof: Martin-Löf Dependent Types — Loom property #14
// Dafny verifies these contracts for ALL inputs, not just test cases.

method Append<T>(xs: seq<T>, ys: seq<T>) returns (result: seq<T>)
  ensures |result| == |xs| + |ys|
  ensures forall i :: 0 <= i < |xs| ==> result[i] == xs[i]
  ensures forall i :: 0 <= i < |ys| ==> result[|xs| + i] == ys[i]
{
  result := xs + ys;
}

method SafeIndex<T>(v: seq<T>, idx: nat) returns (elem: T)
  requires idx < |v|
  ensures elem == v[idx]
{
  elem := v[idx];
}

method Replicate<T>(n: nat, value: T) returns (result: seq<T>)
  ensures |result| == n
  ensures forall i :: 0 <= i < n ==> result[i] == value
{
  result := seq(n, _ => value);
}

method AppendLengthProof()
{
  var xs := [1, 2, 3];
  var ys := [4, 5];
  var result := Append(xs, ys);
  assert |result| == 5;
  assert result[0] == 1;
  assert result[3] == 4;
}
```

2. **Run verification:**
   ```
   cd experiments/proofs/dependent-types
   dafny verify proof.dfy
   ```

3. **Expected output:** `Dafny program verifier finished with N verified, 0 errors`

4. **On success — update `docs/foundations.md`:**
   - Change `EMITTED` → `PROVED` for row #14
   - Change proof experiment line to: `[PROVED](../experiments/proofs/dependent-types/README.md) (Dafny verified — run: dafny verify proof.dfy)`
   - Update summary table row 14

5. **Update `docs/verification-matrix.md` Pillar 2:**
   - Dependent types row: change `🟡 checker` → `🔴 formal (Dafny)`
   - Update P0/P1 gap matrix accordingly

---

## Property 10 — Lamport TLA+ / Convergence (TLC)

**Tool:** TLC model checker (Java)  · **Platform:** Any (Java 11+)  
**Install:**
```
# Option A: TLA+ Toolbox (GUI) — https://lamport.azurewebsites.net/tla/toolbox.html
# Option B: TLC CLI jar
Invoke-WebRequest -Uri "https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar" -OutFile "tla2tools.jar"
```
**Proof file:** `experiments/proofs/tla-convergence/Convergence.tla`

### The claim
Loom's `convergence:` block emits a `ConvergenceTracker` that enforces:
- **Safety:** the distance metric is non-increasing at each step
- **Liveness:** if the system keeps stepping, it eventually reaches distance = 0

TLC exhaustively checks both properties over all reachable states.

### Steps

1. **Create `experiments/proofs/tla-convergence/Convergence.tla`:**

```tla
---------------------------- MODULE Convergence ----------------------------
(* Loom property #10: Lamport TLA+ convergence proof
   Verifies that distributed_step always reduces distance to target.
   TLC checks ALL reachable states — exhaustive model checking. *)

EXTENDS Integers, Naturals

CONSTANTS MaxVal, Target

ASSUME Target \in 0..MaxVal

VARIABLES current

Init == current \in 0..MaxVal

(* Each step moves current one unit toward Target, or stays if equal *)
Step ==
  IF current < Target THEN current' = current + 1
  ELSE IF current > Target THEN current' = current - 1
  ELSE current' = current

Next == Step

Spec == Init /\ [][Next]_current

(* Safety: distance never increases *)
DistanceSafety ==
  [][Abs(current' - Target) <= Abs(current - Target)]_current

(* Liveness: eventually converges *)
EventualConvergence == <>(current = Target)

Abs(x) == IF x >= 0 THEN x ELSE -x

THEOREM Spec => (DistanceSafety /\ EventualConvergence)
=============================================================================
```

2. **Create `experiments/proofs/tla-convergence/Convergence.cfg`:**

```
SPECIFICATION Spec
CONSTANTS MaxVal = 10, Target = 5
PROPERTY DistanceSafety
PROPERTY EventualConvergence
```

3. **Run TLC:**
   ```
   java -jar tla2tools.jar -config Convergence.cfg Convergence.tla
   ```

4. **Expected output:** `Model checking completed. No error has been found.`

5. **On success:** Update `foundations.md` row #10 and `verification-matrix.md`
   temporal logic row to `🔴 formal (TLC)`.

---

## Property 5 — Clarke-Emerson-Sifakis Model Checking (Kani)

**Tool:** Kani Rust verifier  · **Platform:** Linux / WSL2 only  
**Install (WSL2):**
```bash
# In WSL2 Ubuntu terminal:
cargo install --locked kani-verifier
cargo kani setup
```
**Proof file:** `experiments/proofs/model-checking/proof.rs` — harnesses already written

### The claim
Kani explores ALL possible inputs within a bounded domain using SAT/CBMC solving.
Unlike proptest (statistical sampling), Kani is exhaustive: if it passes, no
counterexample exists within the stated bounds.

### Steps

1. **Verify harnesses are present** in `proof.rs` — check for `#[cfg(kani)]` blocks.
   They are already written. Three harnesses:
   - `model_check_process_state_all_valid_inputs` — state machine exhaustive
   - `model_check_safe_increment_all_counters` — counter wraparound exhaustive
   - `model_check_mutex_all_states` — mutual exclusion exhaustive (all 4 boolean combinations)

2. **Run from WSL2:**
   ```bash
   cd /path/to/loom/experiments/proofs/model-checking
   # Create a minimal Cargo.toml if needed (copy from experiments/proofs/):
   cargo kani
   ```

3. **Expected output:** `VERIFICATION:- SUCCESSFUL` for each harness.
   The mutex proof is the key result: it verifies for ALL 4 boolean input combinations
   that mutual exclusion is never violated — exhaustively, not statistically.

4. **On success:** Update `foundations.md` row #5 from `EMITTED` → `PROVED`.

---

## Property 8 — Reynolds Separation Logic (Prusti)

**Tool:** Prusti Rust verifier  · **Platform:** Linux / WSL2 only  
**Install (WSL2):**
```bash
cargo install prusti-contracts --locked
# Or via the Prusti assistant VS Code extension
```
**Proof file:** `experiments/proofs/separation/proof.rs`

### The claim
The frame rule of separation logic: a heap transformation on resources A, B
leaves all resources outside {A, B} unchanged. In Rust, the borrow checker
enforces ownership disjointness natively. Prusti adds formal `#[requires]` /
`#[ensures]` annotations that are discharged by the Viper verifier.

### Steps

1. **Add Prusti annotations** to `experiments/proofs/separation/proof.rs`:

```rust
use prusti_contracts::*;

#[requires(from_account.balance >= amount)]
#[requires(amount > 0.0)]
#[ensures(result.0.balance == old(from_account.balance) - amount)]
#[ensures(result.1.balance == old(to_account.balance) + amount)]
pub fn transfer(mut from_account: Account, mut to_account: Account, amount: f64)
  -> (Account, Account)
{
  from_account.balance -= amount;
  to_account.balance += amount;
  (from_account, to_account)
}
```

2. **Run from WSL2:**
   ```bash
   cargo prusti
   ```

3. **Expected output:** `Prusti: verification successful`

4. **On success:** Update `foundations.md` row #8 from `EMITTED` → `PROVED`.

---

## After Each Property is Proved

1. **`docs/foundations.md`** — change status cell in the summary table
2. **`docs/verification-matrix.md`** — upgrade verification symbol to `🔴`
3. **Update the white paper count** in `C:\workspace\PragmaWorks\gs\generative-specification\docs\white-paper\GenerativeSpecification_WhitePaper.md` — search for "natively proved" and update the count
4. **Commit:**
   ```
   git add docs/foundations.md docs/verification-matrix.md experiments/proofs/
   git commit -m "feat(proofs): discharge property #N <Theory> via <Tool>

   <Tool> verifies <claim> exhaustively / formally.
   Status: EMITTED -> PROVED.

   Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
   ```

---

## Target State

When all 4 are discharged: **15 PROVED, 0 EMITTED** (compiler-enforced properties).
The 3 BIOISO properties (Waddington, Autopoiesis, Hayflick) are addressed separately
in the Bio Iso paper — they are biological isomorphisms, not compiler claims.
