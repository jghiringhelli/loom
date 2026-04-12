// proof.rs — emitted by: loom compile proof.loom
// Theory: Dependent Types (Martin-Löf 1975)
// Types that depend on VALUES. Vec<N, T> carries its length N in the type.
// Rust encoding: const generics are the closest available analog.
// Formal proof: Dafny stubs emitted below.

// ── Rust encoding via const generics ─────────────────────────────────────────

/// Length-indexed vector: the length N is a compile-time constant.
/// This is the Rust approximation of a full dependent type.
pub struct LVec<const N: usize, T> {
    items: [T; N],
}

impl<const N: usize, T: Copy + Default> LVec<N, T> {
    pub fn new(items: [T; N]) -> Self {
        Self { items }
    }

    pub fn len(&self) -> usize { N }

    /// Safe index: Rust verifies idx < N at runtime; Dafny verifies at compile time.
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.items.get(idx)
    }
}

/// Append: Vec<N, T> ++ Vec<M, T> -> Vec<N+M, T>
/// Stable Rust: const arithmetic on generic params requires nightly (`generic_const_exprs`).
/// The runtime version demonstrates the length-in-type invariant; the Dafny stub below
/// carries the full dependent-type proof at the type level.
pub fn append_to_vec<const N: usize, const M: usize, T: Copy>(
    xs: &LVec<N, T>,
    ys: &LVec<M, T>,
) -> Vec<T> {
    let mut result = Vec::with_capacity(N + M);
    result.extend_from_slice(&xs.items);
    result.extend_from_slice(&ys.items);
    debug_assert_eq!(result.len(), N + M, "dependent type: append length = N + M");
    result
}

/// Replicate: produce a vector of exactly N copies of value x
pub fn replicate<const N: usize, T: Copy>(value: T) -> LVec<N, T> {
    LVec { items: [value; N] }
}

// ── Dafny stub (emitted for formal verification) ──────────────────────────────
// Run: dafny verify proof.dfy
// See: https://dafny.org/
/*
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
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_is_in_type() {
        let v: LVec<3, i32> = LVec::new([1, 2, 3]);
        assert_eq!(v.len(), 3);
        // The compiler would reject: LVec::new([1, 2]) where LVec<3> is expected
    }

    #[test]
    fn append_length_is_sum() {
        let xs: LVec<3, i32> = LVec::new([1, 2, 3]);
        let ys: LVec<2, i32> = LVec::new([4, 5]);
        let result = append_to_vec(&xs, &ys);
        assert_eq!(result.len(), 5, "dependent type: length must be N+M = 3+2 = 5");
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn replicate_exact_length() {
        let v: LVec<4, u8> = replicate(0u8);
        assert_eq!(v.len(), 4, "replicate must produce exactly N elements");
    }

    #[test]
    fn safe_index_no_panic() {
        let v: LVec<3, &str> = LVec::new(["a", "b", "c"]);
        // In-bounds access: always safe
        assert_eq!(v.get(0), Some(&"a"));
        assert_eq!(v.get(2), Some(&"c"));
        // Out-of-bounds: returns None (Dafny would make this a type error)
        assert_eq!(v.get(3), None);
    }
}
