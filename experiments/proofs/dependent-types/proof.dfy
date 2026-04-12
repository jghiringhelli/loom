// Proof: Martin-Löf Dependent Types — Loom property #14
// Dafny verifies these contracts for ALL inputs via proof search.
// Run: dafny verify proof.dfy

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
  // Quantifier witness: result[|xs|+i] == ys[i] for i=0 → result[3] == ys[0] == 4
  assert result[|xs| + 0] == ys[0];
}
