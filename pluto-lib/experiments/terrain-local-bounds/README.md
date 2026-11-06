# Cell-local validation cost candidate

**Excluded, unexecuted Rust comparison.** Production, manifests, guards, the
successful historical CPU fixture, and the prepared full-mesh profile are
unchanged. This is not a new cache, public API, installed optimization, or
measured improvement. Eleven pure Python source checks pass; all fourteen prepared
Rust tests and all timing are unexecuted.

## Concrete duplicated work

The exact pinned `Batch` has private `[Quad; 12]` and length fields. Its only
production creation paths are `build`, `build_oriented`, and derived `Clone`.
It exposes only an immutable slice and an emptiness query. There is no unsafe
code, deserialization, mutable reference, interior mutability, or public push
entry point in that module.

`build` validates every supplied UV and extent before returning a value. Origin
is an unsigned coordinate at most 15. All generated positions are finite and
nonnegative, and every UV is finite in `[0,1]`. Side inset is in `[0,0.5)`.
`build_oriented(Top)` only reflects y within the same cell, swaps Up/Down and
reverses vertex/UV pairs. With these bounded inputs, that reflection preserves
the finite, nonnegative-position invariant. `Clone` preserves the value.

The current `cell_local` rechecks finite, lower and upper bounds of all three
position components and both UV components for every active vertex. The
excluded candidate retains only the position upper-bound check. It must never
be applied to arbitrary vertex arrays or to a future constructor merely because
it returns a type with the same name. Full module hashes prevent silently
extending this proof to a changed builder, mutation route or unsafe code.

For a completely local six-face batch the source traversals inspect 120 float
components in the original predicate versus 72 in the candidate, with simpler
predicates. Twelve paired faces inspect 240 versus 144. These are source-level
iteration counts, **not** compiler instruction counts or timing results. An
early rejection can traverse fewer elements in either implementation. The
compiler may transform both loops, and meshing includes much other work.

No `origin == 0` acceptance shortcut or `origin != 0` rejection shortcut is
used. A West-only face at x=1 is valid even when origin.x=1. A Down-only plane
at y=1 can be local before Top reflection and nonlocal afterward. An Up-only
plane can do the opposite. Empty geometry is local at every valid origin.

## Exact source and prepared tests

`source.py` emits only a compact JSON hash receipt. It composes Rust in memory:
the complete unchanged production geometry module, the unchanged original
predicate extracted between exact boundaries, the candidate, and `fixture.rs`.
The whole production files and experiment sources are checked before and after;
symlink components and source drift reject. It launches no tools and writes no
generated tree. Inputs are expected to be quiescent; it is not a general
concurrent hostile-filesystem verifier.

The seven original geometry tests remain intact. Seven extra tests prepare:

1. Every face mask, 0/1 origin corner, diagonal, front/back choice and orientation.
2. Empty output at all 4,096 section origins.
3. Explicit nonzero-origin admitted cases and Top answer changes.
4. Signed zero, exact one, subnormal heights/UV bounds and inset boundaries.
5. A fixed 4,096-case float-bit sequence and actual Clone equivalence.
6. NaN/infinity, negative, above-one, invalid inset and invalid origin rejection
   before any `Batch` is returned.
7. Four bounded paired microtimings on actual batches, with black-boxed inputs,
   seven alternating samples, 128 warmups and 4,096 iterations per sample.

Every comparison first asserts the actual constructed value's finite/nonnegative
position and unit-UV invariants, then compares both predicates. Sampling and
source reasoning are not an exhaustive Rust floating-point proof. The original
predicate stays the executable oracle. Timing shorter than 1 ms or 100 observed
empty-clock intervals is marked inconclusive; that observation is not a
guarantee of clock resolution. No speed assertion or favorable auto-tuning is
present. Future opt0/opt3 runs require actual compiler flags and artifact seals,
not a self-reported profile label.

The generated source is 24,080 bytes with SHA-256
`c8efa6a8fc8b3293174ed7f33fd4ff8da38ae74405dec11475e8a10e2a9bd44b`.
The fourteen tests are only a source inventory. No compiler, native loader,
renderer, current official plan, GPU, or live Game has run this candidate.

## Promotion gates

Do not edit production from this source result. First run the independently
sealed Rust comparison and interpret both optimization profiles honestly. If it
is useful, a separate minimal production delta must also pass the current
official complete plan/guards/type checks and the unchanged whole-output,
fallback/scratch replay and end-to-end profile fixtures. Any broader constructor
change requires redoing the invariant review. Microtiming cannot establish
mesh-level speedup or repair missing consumer startup/model registration.

Source self-review corrected a missing candidate closing brace before remote
publication and added a small boundary regression. The pure source checks are
not a Rust parser or compiler; Rust typing and execution remain unverified.
