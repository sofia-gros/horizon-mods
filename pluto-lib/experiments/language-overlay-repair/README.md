# Bounded language overlay repair candidate

Excluded, unapplied Pluto internal candidate on official Horizon f4dcf8a2.
This is not in the ordinary package, manifest, public API, or CI. No official
checkout is changed. The latest Pack8 baseline and its successful trigger remain
unchanged. Fourteen Rust tests are prepared, **not compiled or executed**.

PlutoLib owns reusable resource preparation; the native pack owner still chooses
selected/server order and filters, the admitted native Content owner still
provides Mod files, and native Language/CURRENT remains the sole lookup owner.
This does not add a translation registry, locale option, formatter, or callback.

## Exact intended change

At the admitted-Mod writer in native `mod_content::build`, after its package
reader succeeded and before `Overlay::write`, `prepare` reads only the matching
file in the already filtered, prepared absolute base root. It never reads raw
packs, the mutable output root, or another Mod's source. The native namespace
clash check and Plan construction still occur before output recreation.

This lower=base rule requires native-admitted, subsequently unmodified Content.
Official `Content::check` permits language assets only in owned namespaces;
its shared-namespace exceptions are data tags and Minecraft atlases, not lang.
Then `build` rejects overlapping namespace owners before any output recreation.
Thus another admitted Mod cannot already have written the same locale path.
The public build API does not independently revalidate arbitrary hand-constructed
or later-mutated Content; this candidate does not promise that broader contract.
The exact admission prefix, full content.rs, and build source are pinned.

Only `assets/<namespace>/lang/<locale>.json` with the documented ASCII safe
namespace/locale path is eligible. Both layers must be complete strict JSON
objects of unique string keys and string values. Escaped duplicate keys are also
rejected. Each input is at most 1 MiB; each map and final union at most 16,384
entries; decoded union text and encoded output at most 2 MiB. These are candidate
compatibility limits, not native global language limits. Input byte bounds limit
parser allocation, but they are not a process-memory/OOM guarantee.

The resulting map retains lower-only keys and lets Mod values win every conflict.
No placeholder, Unicode value or language fallback is interpreted. A strict empty
Mod object now retains lower keys: this is an intentional policy change. Native
pack lenient output can qualify after the pack owner normalized it; raw lenient
Mod bytes do not. Strict numeric/boolean/object values remain unsupported.

Out-of-domain paths, either invalid/duplicate/over-budget layer, lower read errors
or absent lower files, encoding limits, and zero additional lower keys preserve
the **exact original higher byte slice**. The native admitted-package read error
still occurs before the candidate and is not swallowed. Optional added lower
reads fail back to the original native write rather than creating a new error.
The base is assumed to be quiescent, as required by the resource preparation
owner; there is no new filesystem transaction or adversarial-race guarantee.

The Cow is scoped and dropped before native later use/move of the original bytes.
The original `Overlay::write` still materializes directories, detaches the old
file/link and writes in its original order. No post-build walk edits that root,
and no existing base or cache file is rewritten by the helper.

## Cache policy and incomplete connection

An inseparable second proposed seam adds the fixed `POLICY_ID` as one native
length-prefixed stamp hash input immediately after the original domain. All
native metadata inputs, their relative order, original fallible operations and
hex output shape stay unchanged. A suffix on the final stamp would not change
its first-16-character directory and is deliberately not used.

This fixed discriminator can rebuild non-language Mod roots too. It is not
per-locale detection or proof that every filesystem mutation changes a stamp.
Original size/mtime/native-prefix collision limitations remain. Do not prune or
manually invalidate caches to make a test pass. Cache-hit bypass of build means
the writer alone cannot safely activate this policy on an existing root.

`source.py` returns a candidate source in memory: exactly one writer replacement,
one stamp insertion, and the private helper module. Removing those deltas must
reproduce every byte of the official source. This is an **unapplied prototype**,
not proof that current Mixins can express the two sites with correct borrows,
module placement and exclusive full installed composition. It does not bypass
that missing placement gate, nor suggest overwriting a large host function.

Existing native serde/serde_json dependencies suffice; no runtime dependency is
added. The local parser is a temporary preparation value, not a lookup owner.
Actual native typing and all fourteen tests remain unrun. Selected source hashes
are not a clean full checkout, compiler, or complete composition proof.

## Next distinct validation gates

1. Source/inverse checks and independent review of strict domain/failure ordering.
2. Actual latest Mixins placement and all-installed composition for both seams.
   If unsupported, report the blocker; do not install only the writer.
3. Separate test-only latest native copy with exactly these deltas: actual
   Package/Content/packs/build comparisons must differ from Pack8 only at intended
   lower-only retention. Include filters, conflicts, unchanged higher bytes,
   raw lenient/duplicate/over-limit fallbacks and namespace/read-error order.
4. Actual native resource_root with fixture-owned caches: fixed old/new policy
   inputs select different sample roots, same-policy reuse, retained old-root
   readability, failure/pruning boundaries. No process-global cache alteration.
5. Game applied/live-Mod and Options/locale installation, retained Guide/LAN,
   actual UI refresh and production data remain separate, unproved work.

Python source checks may run with `PLUTO_SELECTED_SOURCE` pointing at the exact
selected official files. They perform no compiler/planner/Cargo call or source
output. Rust execution requires a separately reviewed runner and serial grant.

## Source checkpoint

Thirteen pure Python source/inverse checks passed. The in-memory candidate is
129,946 bytes, SHA-256 `1c3ebe1c8bf56ae6a2aab926bd6704d3b1ee40e1028eb32928ea0b76a2eea71e`.
Independent read-only review found no concrete serious type/borrow/order blocker.
This is not native compilation or execution of the fourteen Rust cases.
See `source-validation.json` for exact selected-source and check scope.
