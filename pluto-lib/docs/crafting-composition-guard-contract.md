# Crafting composition guard contract

Source-only follow-up to ordinary runtime fbf6f16f3348f385320743df3b6e20d6d3e6f7bc.
No Rust, native plan, apply, Windows stage or host type check was run for this
change. In particular the two new Recipes constructor nodes and function
fingerprints remain null. Full Tinkers checking must reject until a fresh actual
plan supplies those exact observations and the reviewed metadata is updated.
The prior actual 153 API / 16 finite tests do not prove this startup integration,
pure pair, station transaction or complete composition.

## Exact entry points and separate modes

The existing `check_crafting_match_plan.py` and `check_recipe_placement_plan.py`
remain mandatory. The placement-definition boundary is retained, not replaced
by the local transaction boundary. Shared `check_order` / `verify_source`
functions keep their legacy call signatures for station ingredient checking.

* `standalone-pluto` declares exactly the Pluto bundle and four sole-owner HEADs:
  matching/derivation, remainders, placement definition and local placement.
  Eight pinned modules in the two protected native files retain native fallback
  when no provider is registered. TConstruct is not a required library dependency.
* `full-tconstruct` explicitly declares Pluto plus TConstruct. It adds the two
  noncancellable constructor startup HEADs, for six boundaries. Sixteen pinned
  dependency modules, the two station session states and the independent LAN
  station hook must remain exact. Whole protected-file hook/module sets reject
  unknown sibling hooks, alternate constructor hooks, bridges and state changes.
  Full Tinkers cannot silently fall back to standalone when any consumer is absent.

The full contract preserves distinct authorities: matching remains deny-only;
Overslime's source-pinned pair provider recomputes independently for assembly and
remainders; station transaction handling follows admission and owns a claimed
invocation without rerunning the native body. The guard does not turn those
separate responsibilities into a one-call pair transaction, persistence identity,
rollback or live actor/network proof. The unchanged old Overslime files and the
station transaction's complete old body remain in the ordinary tree.

## Caller protocol: required adoption dependency

All three CLIs now take:

    CHECKER COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE CRAFTING_PACKAGE_CONTRACT.json

This includes `check_native_plan.py`, which forwards the same third path to both
crafting-related guards; the other ten guards retain their own signatures.
Storage's direct twelve-guard wrapper must pass it to those two guards too.
Windows `stage.py` must generate and pass it in the same adoption unit. Replacing
only the guards while leaving a two-argument stage caller would fail closed at
usage validation. No silent default or environment-based mode inference is used.

After checking its immutable expected package hashes, the runner imports the
existing `check_recipe_placement_plan.py` and calls:

    make_package_contract('full-tconstruct', {
        'pluto_native_creative': absolute_path_to_actual_pluto_hnmod,
        'tconstruct_native': absolute_path_to_actual_tconstruct_hnmod,
    })

The return value is a JSON-compatible dictionary. Write it to a fresh, run-specific
work file and seal that file and its admitted archives before and after guards.
This helper is a recorder, not authorization to select different packages. It
does not infer mode and rejects a missing or extra archive owner. The analogous
standalone call explicitly supplies `standalone-pluto` and only the Pluto archive.

Schema 1 has exactly `schema`, `mode`, `horizon`, `mixins`, `packages`. Each package
has exactly `mod_id`, `version`, `archive` (absolute hnmod file path),
`archive_sha256`, `manifest` (internal ZIP name), `manifest_sha256`.
The official pins are Horizon 41f822b659789780f58d32c63ddfb6887ea1b8bf and
Mixins 5a64af938ec86b00d53d16a408aabc0038fb3e57.
`tools/crafting-package-contract.json` is an intentionally unusable template:
null paths/digests are not actual archive evidence and always reject.

The guards independently read back bounded archives and manifests. They verify
identity/version/revision, the exact declared owner set and every manifest
injection/module/state/crate/bridge ID against the complete plan. All packaged
module/helper bodies and all packaged crate file hashes must equal the plan's
embedded bodies/files. Thus changing a package payload without changing the plan
cannot pass by updating only the outer archive digest. Duplicate member names,
unlisted manifests, symlink/encrypted/directory members, traversal and excess
member/size budgets reject. No archive is extracted by these checks.

A schema-6 plan does not itself record empty declared Mod manifests, hence the
explicit package evidence is necessary; looking for a missing TConstruct helper
would be an unsafe mode selector. The API crate record and its entire file set
are pinned to one `pluto-inventory-api` identity, including the stored matching
extension. Native dependency files are rehashed against `plan.inputs`.

## Source validation and limits

Forty-three source/synthetic checks passed locally: ten existing boundary/source
checks, ten existing dispatcher checks and twenty-three new contract regressions.
The two existing actual-package tests are retained and extended to include the
new delegation modules, but were deliberately not executed in the small source
view. They remain required in the integrated package gate. No existing source
suite count was lowered to admit a pass. The old HEAD-local literal assertions
were replaced by exact delegate hashes, denial-before-derivation order, explicit
transaction ownership, and complete legacy-body/file preservation checks.

Synthetic archives/plans in these tests exercise mode and rejection behavior;
they are not evidence that a real constructor was selected. The source view uses
unchanged common inputs plus exact ordinary postimage files, not an applied host.
Guide independently reviewed the final guard source and identified the mandatory
stage-caller adoption dependency; no further concrete blocker was found.

Next gates: fresh complete plan; observed constructor IR pinning; both explicit
package guards and all other mandatory guards; actual package source tests; apply
and installed host typing; remaining finite pair/startup/station tests as needed.
No approval to apply or launch follows from this document or the source checks.
