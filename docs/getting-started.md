# Getting Started with Loom

This guide takes you from zero to a working Loom program in 10 minutes.

---

## Prerequisites

- Rust toolchain (stable): https://rustup.rs
- `cargo` on your PATH

```sh
rustup update stable
```

---

## 1. Build the compiler

```sh
git clone https://github.com/pragmaworks/loom
cd loom
cargo build --release
```

The binary is at `target/release/loom` (or `target/release/loom.exe` on Windows).

Add it to your PATH, or use `cargo run --` as a prefix for all commands below.

---

## 2. Your first Loom program

Create `hello.loom`:

```loom
module Hello
describe: "A greeting with a proved contract"

fn add_positive :: Int -> Int -> Int
  require: a > 0
  require: b > 0
  ensure:  result > 0
  a + b
end

test it_works ::
  add_positive(2, 3) = 5
end
```

**Compile to Rust:**

```sh
loom compile hello.loom
```

**Type-check only (no output):**

```sh
loom compile hello.loom --check-only
```

---

## 3. Emit to other targets

```sh
loom compile examples/02-payment-api.loom --target ts        # TypeScript
loom compile examples/02-payment-api.loom --target openapi   # OpenAPI 3.0 YAML
loom compile examples/02-payment-api.loom --target schema    # JSON Schema
loom compile examples/02-payment-api.loom --target wasm      # WebAssembly Text (WAT)
```

---

## 4. Contracts in depth

Every `fn` can have:

| Clause | Meaning | Rust output |
|--------|---------|-------------|
| `require: expr` | Precondition (caller's obligation) | `debug_assert!(expr)` at top of fn |
| `ensure: expr` | Postcondition (function's guarantee) | `debug_assert!(expr)` after body |

Contracts reference function parameters directly. In `ensure:`, use `result` to refer to the return value.

```loom
fn safe_divide :: Float -> Float -> Float
  require: divisor != 0.0
  ensure:  result != 0.0 || numerator = 0.0
  numerator / divisor
end
```

---

## 5. Units of measure

Units prevent mixing `Float<usd>` with `Float<eur>` at compile time:

```loom
type Price    = Float<usd>
type Quantity = Int

fn total_cost :: Price -> Quantity -> Float<usd>
  price * quantity as Float
end
```

---

## 6. Privacy labels

Label fields that carry sensitive data. The checker enforces co-occurrence rules (e.g. `@pii` requires `@gdpr`):

```loom
type Customer = 
  id:    Int,
  email: String @pii @gdpr @never-log,
  card:  String @pci @encrypt-at-rest
end
```

---

## 7. Effect tracking

Declare what side effects a function has:

```loom
fn fetch_user  :: Int  -> Effect<[IO, DB], User]     -- reads DB and network
fn send_email  :: User -> Effect<[IO@irreversible], Unit]  -- cannot be undone
fn pure_add    :: Int  -> Int -> Int                  -- no effects
```

The effect checker flags calls that ignore declared effects.

---

## 8. Typestate / lifecycle protocols

Enforce valid state transitions at the type level:

```loom
lifecycle Payment :: Pending -> Authorized -> Captured | Voided
```

A function accepting `Captured` cannot receive a `Pending` payment — it is a type error.

---

## 9. Multi-module projects

Create `loom.toml`:

```toml
[project]
name    = "my-service"
version = "0.1.0"

[[modules]]
path = "src/user.loom"

[[modules]]
path = "src/payment.loom"
```

Build all modules:

```sh
loom build
```

---

## 10. The LPN AI-to-AI protocol

For AI agents orchestrating multiple compilation steps, use `.lp` files:

```lp
# hello.lp
EMIT rust PaymentAPI FROM examples/02-payment-api.loom
CHECK all examples/02-payment-api.loom
VERIFY contracts examples/01-hello-contracts.loom
```

```sh
loom lpn hello.lp
loom lpn hello.lp --format json   # machine-readable
```

LPN instructions are unambiguous, token-efficient, and fully typed — designed so AI agents can read and write them without ambiguity.

---

## Next steps

| Goal | Where to look |
|------|--------------|
| Full language syntax | [`docs/language-spec.md`](language-spec.md) |
| All 116 milestones | [`docs/roadmap.md`](roadmap.md) |
| Compiler architecture | [`docs/TechSpec.md`](TechSpec.md) |
| Example programs | [`examples/`](../examples/) |
| Contributing | [`CONTRIBUTING.md`](../CONTRIBUTING.md) |

---

## Troubleshooting

**`error: could not find loom in PATH`**  
Add `target/release` to your PATH or prefix commands with `./target/release/loom`.

**`parse error: unexpected token`**  
Loom uses `=` for equality in expressions (not `==`). Use `a = b` for equality checks.

**`type error: effect mismatch`**  
A function returning `Effect<[DB], T]` cannot be used in a pure context. Either declare the caller's effects to include `DB`, or use an effect adapter.

**`checker error: @pii without @gdpr`**  
All `@pii` fields must also carry `@gdpr`. Add `@gdpr` to the field.
