# Language writer/stamp placement: rejected on latest official

[Run 42214736963](https://github.com/sofia-gros/horizon-mods/actions/runs/42214736963),
job 127881810684, attempt 1, exact source
`150acf22dda956979805da9d3a26c8a4feee8654`.

**The diagnostic completed, but the placement was not accepted.** The green
workflow conclusion means the deliberately negative diagnostic was retained,
not that this repair works. Final receipt reports `probe_completed=true`,
`source_verified=true`, `plan_accepted=false`, `error=null`.

The exact official Horizon f4dc / Mixins 5a64 tool returned after 7.064 seconds:

- `E_UNSUPPORTED_CONTROL_FLOW`
- Injection `language-policy-stamp`
- Target `horizon_client_lib::mod_content::stamp`
- `nested items in target functions are unsupported`

No plan file or selected IR was produced. Consequently this run establishes
neither the separate writer HEAD's acceptance nor the helper's native types.
There was no apply, helper14 execution, real writer/build/cache, Game/UI or
complete installed-composition check. Candidate source and ordinary runtime
remain unchanged; there was no automatic retry.

## What did run

- Eighteen runner, thirteen repair-source and twelve placement-source checks.
- Exact two official checkouts/locks and seven language source pins.
- Cold compile of the actual official Mixins CLI, 146.577 seconds.
- One metadata-only/read-only plan. The recorded Cargo call was metadata with
  offline/locked/no-deps, not a host build. Direct CLI CPU observation ended
  consistently with CLI exit 1.
- Full declared source/Git/payload/wrapper/CLI seals verified. Minimum free
  90,683,207,680 bytes; minimum admitted available memory 13,648,277,504 bytes.

The complete bounded job console and final receipt are preserved beside this
document. Console 35,000 bytes, SHA256
`ab88d652167cc0e42b28c9509f37bb6be4a08f2b48461b815f46fbf3dcce0611`.
The diagnostic phase had 302 bytes and SHA256
`60f30fa61d9e34b240735166527ac6dca73935ca1477be0ddc32d8e36f0987d3`.
Other raw compiler phase files were ephemeral; a recorded hash is not a promise
that their full bytes can be downloaded later.

## Source explanation and RETURN alternative

The exact official [stamp function](https://github.com/horizon-rust/horizon/blob/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd/crates/horizon-client/src/mod_content.rs#L2830-L2877)
has a function-local `use sha2::{Digest, Sha256};` item and a metadata closure.
It does not contain a nested function definition. The diagnostic says nested
**items**, which must not be described as proof of a nested function.

In exact Mixins [validate_target](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L919-L980),
`boundary` is true only for HEAD. The [visit_item rule](https://github.com/horizon-rust/horizon-mixins/blob/5a64af938ec86b00d53d16a408aabc0038fb3e57/crates/horizon-mixins-core/src/plan.rs#L1176-L1187)
rejects function-local items otherwise. `rewrites_body=false` for RETURN does
not waive that rule. RETURN also does not allow the function's `?` exits through
`try_outside_value`, which is reserved to MODIFY_VARIABLE.

Therefore a RETURN transformation rehashing native stamp plus policy is not a
viable same-target alternative under this exact validator. This conclusion is
source-derived, not another executed RETURN probe. No wide stamp-body copy,
caller rewrite, guard relaxation or official edit was substituted. A future
small caller boundary requires separate source, lifecycle/error-order, cache
identity and actual-placement evidence before adoption.

## Lossless receipt transcription correction

The earlier `ec7e7717` JSON rounded `/final_receipt/tool_argv/mtime_ns`
from the raw integer `1793966774363156664` to `1793966774363156700`
through JavaScript Number. This follow-up reparses the unchanged saved console
with Python `json.loads` and restores the original integer, without changing it
to a string. Recursive comparison found this one difference only. The exact
35,000-byte console and its SHA256 remain unchanged, as do all diagnostic
conclusions. Future receipt transcription preserves integer values in Python.
