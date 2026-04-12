# Proof: Gradual Typing (Jeremy Siek & Walid Taha, 2006)

**Theory:** A gradual type system smoothly combines static and dynamic typing. Static regions enjoy full compile-time verification; dynamic regions use runtime checks. The key guarantee: static types are *never silently bypassed* — every use of a dynamic value in a static context generates an explicit, checked cast.  
**Claim:** Loom's `gradual:` block creates explicit dynamic regions. Outside them, all types are statically verified. Dynamic values used as concrete types generate checked casts that fail explicitly, never silently.

## What is being proved

**The gradual guarantee:** If a program has no `gradual:` blocks, it is fully statically typed. Adding `gradual:` blocks introduces exactly as much dynamism as declared — no more. The static/dynamic boundary is explicit and auditable.

**Correct:** Dynamic value cast to correct type → succeeds.  
**Failure case:** Dynamic value cast to wrong type → explicit `None`/error, never silent corruption.

## How to run

```bash
loom compile proof.loom -o proof.rs
cargo test
```

Expected:
```
test static_region_fully_type_safe ... ok
test gradual_region_accepts_dynamic ... ok
test checked_cast_succeeds_for_correct_type ... ok
test checked_cast_fails_gracefully_for_wrong_type ... ok
test pipeline_bridges_static_and_gradual ... ok
test static_types_never_silently_bypassed ... ok
```

## Layman explanation

Like a building with an airlock: the static wing is fully sterile (type-safe). To bring something from outside (dynamic data), you go through the airlock (checked cast). The airlock verifies what you're bringing in. If it's wrong, the airlock rejects it — it never lets contamination through silently. Gradual typing is the formal theory that proves you can design a language with this property while still being useful for real-world code that mixes typed and untyped parts.
