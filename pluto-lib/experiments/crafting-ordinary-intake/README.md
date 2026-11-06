# Crafting and station ordinary postimage

This is a **source composition candidate**, not installed or validated native
gameplay. Base: `59c23ce300e40eb6a477bb55f33552fddbc04599`, Horizon
`41f822b659789780f58d32c63ddfb6887ea1b8bf`, Mixins `5a64af93`.
The later integration documentation-only changes do not alter these ordinary
inputs. Cloud and three incoming-payload migrations in this base are preserved.

`compose.py` produces exactly 16 ordinary files. `before.json` fixes the 14
existing inputs by content; immutable connector Git blob identities verified the
12 code files and the coordinator supplied the exact two current manifests.
TConstruct manifest SHA is f9356fc8c3925bbe77930f7648bdbc4fff9fd2925680d0791d6b8c1c17b1ff62;
Pluto manifest SHA is c0f312d906a2d42249d167663ffce771480adbefa1cae0f1745cbb78aad9e040.
`proposals.json` contains the exact already-reviewed proposal bodies named and
hashed in inputs.json, not another active API copy. The sole new combined startup
implementation is combined-bootstrap.rs. Source-only composition changes no
checkout and creates no CI trigger.

## Exactly delegated old hooks

Only these five old injection records are removed in the proposed postimage:

- anvil-material-recipe and survival-melter-no-container: the successful stored
  matching API/reader and one TConstruct policy registration. API153 and finite16
  passed at run42247376045. Policy order and stored-field semantics are unchanged.
- overslime-crafting-assemble and overslime-crafting-remainders: the separate
  source-pinned native pure-pair provider. Assembly still compresses nonempty
  cells; remainders still use original positions and exact ID. They recompute
  independently. This migration adds no once-only pair-consumption guarantee.
- crafting-station-recipe-placement-guard: the complete unchanged signature/body
  follows a static registration prefix. The sole Pluto HEAD first runs existing
  veto/preview, then one claimed native transaction, and never reruns native
  placement after that claim.

All original helper/policy files remain present and unchanged, including the
whole Overslime plan/amount/refill owners. Their hashes are in receipt.json. The
separate LAN station hook and every unrelated injection remain unchanged.
No feature is deleted merely to satisfy exclusive-target guards.

## Initialization and dependency direction

Two noncancellable HEADs at actual library-owned Recipes::empty/from_sources call
one shared OnceLock. The prior excluded fragment used horizon_client; this
postimage explicitly uses **horizon_client_lib**, consistent with the actual
recipe owner. Their native node/fingerprint/binding acceptance is still unobserved
and must come from a fresh actual plan, not a guessed metadata value.

Initialization registers the stored denial pair first and the pure derivation
provider second, caching their typed Result together. Normal error is cached
before a deliberate panic because Recipes::empty cannot return a load error. If
the second registration fails, the first remains inserted. There is no rollback.
Both guarded constructors fail rather than return a new partially protected table.
Same-thread recursive initialization is unsupported and the source-pinned static
registration functions invoke neither callbacks nor Recipes. Unexpected initializer
panic leaves std OnceLock unset; a later attempt must still obey duplicate checks.
Caller effects before entry, including remote incoming-data take, are not restored.
Five new helper test bodies cover ordered success, first/second failure, parallel
calls and panic/partial state; they are prepared but **not executed**.

Station registration is separate and Game-side, after the existing remote/LAN
branches and before local entity/grid/session publication in open. This avoids
reversing the Game-to-client-library dependency direction. Its cached-error panic
does not undo prior ensure_datapacks effects. Hot adoption into an already-open
unregistered session is unsupported. The handler contract prohibits Game-level
place_recipe reentry; the actual old body calls only menu.place_recipe. No new TLS,
actor cache, item reconstruction or mutation from admission is introduced.

## Source inverse and policy preservation

The source-level station accessors remain `__horizon_mixins_tconstruct_*`.
Comparing them with the old actual plan requires only the established package
conversion to `__horizon_mixins_tconstruct_native_*`; generated names must not be
written back into ordinary source. The raw station postimage adds exactly one
registration line and inverts exactly. Its transaction body matches the actual
plan's helper verbatim.

The manifest removal markers restore every old injection byte-for-byte. The
existing Pluto click changes only by adding its actual use_max formal. All old
module entries remain exact; six new modules and three new hooks are added, five
delegated hooks removed. Non-module/injection tables, including states, crates,
compatibility pins, cloud and payload migrations, remain unchanged. Ordinary
source and candidate artifacts must be merged by the central integration writer.

## Required native gates

`guard-contract.json` specifies the new composition requirements. Existing
exclusive guards are not weakened or reported passing. The source postimage has
one owner for assembly, remainders and local placement, but their dispatcher and
startup dependencies require new guard evidence:

1. Review this source postimage and its exact package closure.
2. Obtain a fresh complete plan, observing both constructors and all new helpers,
   modules and unchanged other owners. Keep unknown constructor fingerprints
   pending rather than copying the assembly fingerprint or inventing a value.
3. Fix the mandatory guard metadata/implementation to the actual accepted IR and
   exact source closure, preserving sole ownership and rejecting bypass/reentry
   compositions. Then run all guards before apply.
4. Actual apply and host library/binary typing, constructor startup, the unrun
   pure-pair16 and combined-bootstrap5, and local/LAN transaction behavior remain
   separate proof obligations. The previous API153/finite16 result covers only
   the real API and finite reader/policy fixture, not this installed composition.

Current checks: **13 source/inverse tests PASS**. All old body/source comparisons
above are exact. No new Rust, Cargo metadata, plan, apply, registry startup or
gameplay execution occurred while creating this candidate. Ordinary activation
and complete guard success remain unproved.
