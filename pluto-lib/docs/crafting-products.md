# Explicit crafting output and remainder staging

`pluto_inventory_api::crafting::products` is a synchronous proposal contract.
Its native adapter, `crafting::pluto_products::with_grid`, borrows the caller's
actual complete square grid and current Items. It prepares one owned output
and all owned remainder values together. It does not match recipes, register
providers, intercept crafting, consume slots, publish results, allocate recipe
IDs, cache decisions, or grant inventory authority. The existing matching and
placement vetoes are unchanged and independent.

## Inputs and policy

The adapter preserves native `Input::of` crop geometry, holes and ordering. A
consumer reads host-independent item/component facts via `Preparer::cell` and
submits one `Proposal`. `Source::Input` selects an actual cropped input stack;
`Source::Fresh` selects a known fresh default item. No ItemSnapshot is converted
back to an ItemStack. Recipe selection, material eligibility, quantities,
conservation, output IDs, consumed ingredients and component keys are consumer
policy. The service deliberately cannot infer these rules.

A proposal has exactly one remainder position per cropped cell. Some is allowed
only for an originally occupied position; None means no remainder. Returned
remainders are expanded into the original side-by-side grid, including outside
padding, with no reordering or cloned duplication. Selecting one input multiple
times is an explicit consumer proposal, not proof of conservation or permission
to duplicate a live item.

All structural bounds are checked before any stack is derived: at most 4096
grid cells and 4096 compound patches, 65536 combined field/value nodes, value
depth 64, 1024 UTF-16 units per key/string or UTF-8 bytes per ID, and 4 MiB of
proposal value/text data. Counts must be positive and no larger than the current
item's effective native maximum stack size or i32::MAX. The finished stack
uses Menu's exact valid `minecraft:max_stack_size` override (1..99), removal
marker precedence and catalog fallback, rather than assuming the base item limit. Limits bound submitted proposals, not
arbitrary caller computation or the depth/size of existing native stacks.

## Preserved native values

Input-derived products clone the actual ItemStack, including untouched native
runtime fields. The codec is used to validate save eligibility, never to obtain
the clone. Recursive retained wire anywhere in the complete source grid, including
unselected cells, causes the entire preparation to fail, even
for a count-only proposal. Count-only wire preservation may support a narrower
future contract; it is not generalized to metadata changes here.

A `CompoundPatch` changes explicit top-level fields of a preserved raw
`StackData.other` compound. Unknown compound fields and unrelated native state
remain on the clone. This is not a general setter for modeled components. It
does not use `StackData::set_other`, which has special modeled behavior. Original
input clones and finished proposals must both pass the existing native lossless
saved-item codec check. Every requested component must also encode to the exact
requested Tag after patching. A raw duplicate that the codec canonicalizes,
modeled/conditional absorption, noncompound input or other unrepresentable
change fails the whole pair rather than selecting an assumed owner.

Missing/create is explicit. It is allowed only when the new raw compound
survives the same native checks without losing unrelated state. Patch field
operations are ordered; later changes to the same key replace earlier changes.
Nested replacement values preserve NBT variant identity, integer widths, float
bits and Java UTF-16 units. The inverse component conversion is structural only,
not schema validation or a stack replacement capability.

The saved-item codec intentionally does not compare every runtime-only field.
That does not lose them here: the returned object is the original full clone,
not the decoded validator. The eligibility check is the existing native item-to-Tag/item-from-Tag roundtrip,
not an additional binary-NBT serialization test. Codec eligibility, clone
preservation, slot identity and transport compatibility are distinct claims.
This service does not guarantee arbitrary binary encoding, network serialization
or raw-record replacement safety.

## Atomic preparation, not atomic consumption

`stage` derives output and all remainders privately. If any derivation fails,
all staged values are discarded and the caller receives no partial pair. The
input remains borrowed and unchanged. `Prepared::release_if_unchanged` requires
an explicit current complete input slice and compares it with the retained full
snapshot, including runtime bundle selection and observable component state.
It rejects missing storage, changed counts, geometry/order or any compared
state. Original retained wire is excluded, so no private wire baseline equality
is inferred.

This receipt is a value comparison, not a world, menu, slot or lifetime token.
A same-valued unrelated grid can compare equal. No lock is acquired and a later
mutation can race. The integrating authoritative caller must still validate its
menu/world, all affected storage, matching/denial policy, allocation and packet
rules, and atomically publish/consume at an appropriate boundary. No such
production caller is installed in this step.

## Why automatic hooks are deferred

Official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0 has separate assembly
and remainder calls. `Menu::safe_take` removes the result before `on_take`;
`on_take` clones the input grid, calls recipe find twice, derives remainders,
then decrements inputs and merges/drops remaining items. The crafter similarly
finds an output, performs map postprocessing, derives remainders from the
original grid, ejects products and then decrements inputs.

Separate automatic callbacks can return inconsistent output/remainder pairs,
even with identical immutable facts: sealing a registry does not enforce pure
or deterministic callbacks. This slice therefore adds no assembly/remainder
hooks and does not weaken the existing complete-plan matching guard. Automatic
integration remains blocked on a proven single authoritative pair-consumption
route or a separately justified narrow consumer purity contract. No persistent
recipe ownership or hidden cache is substituted for that proof.

## Consumer evidence and validation scope

The test-only TConstruct proposal keeps all Overslime IDs, selection and refill
rules outside the library. Its reference is the exact MOD source at
8c56bb52b6b77fae6115e894af9e369a5c3f9e08. Its one real metadata operation is a
preserving patch of `minecraft:custom_data` / `tic_overslime`; consumed bottles
become fresh glass bottles and unused bottles remain full input clones with
count one. Tests compare the complete proposed pair with that source behavior.
An independent synthetic `example:` policy covers fresh output plus a preserved
catalyst patch. It demonstrates another policy shape, not certification of an
existing external mod.

The bounded fixture uses exact official Input and real native Items, ItemStack,
component/storage codecs and unchanged shared adapter sources. Inherited recipe
wrappers and special endpoints are modeled and do not add gameplay coverage.
No test here proves native Menu consumption, production TConstruct activation,
LAN delivery, or integration with every original PR. See the dated validation
report for actual completed gates; unrun gates remain explicitly pending.
