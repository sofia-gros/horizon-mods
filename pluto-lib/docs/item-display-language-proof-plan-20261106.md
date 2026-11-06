# Item display and selected-language proof plan

Status: source preparation only. No Cargo metadata, Rust test, native Mixins
plan/apply, Game construction, rendering or network execution has been run for
this fixture. It changes no production API, default component, locale owner,
translation data or existing TConstruct behavior.

Target is official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
The inputs record pins the complete official files, two existing Pluto adapter
sources, the unchanged author example and the eight new fixture tests. Its source-only route hashes are
separate from functions that the fixture actually compiles.

## Existing display contract, not another key registry

`creative::Names::display_item` asks the selected language for `item.NS.PATH`,
then `block.NS.PATH`, then its loaded English map and finally the path with
underscores replaced by spaces. PATH is not normalized from slash to dot.
Unqualified names use the Minecraft namespace. `Names::display_get` differs
from a missing-key-as-text function: it returns `None` if current and English
maps both lack the key.

An explicit `minecraft:item_name` stack component is resolved by
`ItemStack::item_name` through the actual core component parser on every call.
The stored Tag remains translatable. `item_tooltip::styled_name` takes the
custom component first, then item-name component, then its lazy fallback;
custom names inherit italic unless they explicitly override it. The Game plain
fallback has the additional written-book/ominous-banner/potion/player-head
branches. In the actual tooltip call, `styled_name` wraps that fallback. This
plan does not replace those priorities with a new generic precedence policy.

The author-facing `Key.resolve` delegates to the existing internal
`pluto_language` service. Its resulting Text spans are a snapshot, while
`Text::component()` retains the native translatable source. The fixture passes
that component through the existing core JSON-to-Tag conversion into an
explicit item-name patch. It does not store `plain()` as the item name.
No new locale cache, alias table, event registry or translation ownership is
needed for the standard-key path.

## Actual compiled scope

The fixture uses the real official `horizon-items`, `horizon-client-core` and
`horizon-world` packages by exact local Cargo paths. It copies both complete
Pluto language adapter files, including their actual composed reexports, unchanged.
The unchanged Guide example is included for type checking with a fixture-root
`horizon_client` alias. This verifies imports and same-type reexports in this
bounded crate, not acceptance of the installed client-library module manifest. It extracts the complete official
Names struct and impl, and the complete official `styled_name` function byte
for byte. Their dependencies are reexports of actual native modules, including
`content_namespaces::namespaces` and core `component::name_text`, not semantic
stubs. Names loads a tiny explicitly-created resource root through its real
loader. Native `lang::scoped` isolates each test language without writing the
process-wide language setting.

Eight additional tests cover:

1. Selected item before block key; pinned English fallback and optional lookup.
2. Slash preservation, implicit Minecraft namespace and missing-path fallback.
3. Re-reading the same translatable item-name Tag in English and Japanese.
4. Custom-name priority and explicit italic/color preservation.
5. Author Key/Text source through native ItemStack NBT save/load, with arguments
   and styling retained, then a fresh language resolution.
6. Missing key versus an explicit native fallback template.
7. Plain-stack default name delegated exactly once to current Names lookup.
8. `ItemStack::new` does not fabricate an item-name component; an explicitly
   supplied patch is a different case.

The unchanged two adapters contribute 14 existing tests, so the unfiltered
fixture contains 22 tests. A future executor compiles once, lists all exact
names, then executes the complete unfiltered suite once with exactly 22 passed,
zero failed, zero ignored and zero filtered. Its receipt partitions that one
result into three disjoint groups: `item_display_tests::` (item8),
`pluto_language_text::tests::` (Key6), and `pluto_language::tests::` (internal8).
The prefixes classify results, not Cargo execution filters. The names must
partition all22 without omission or overlap. Require actual compile evidence
and all source/lock seals before and after. Do not rerun adapter14 in a second
fixture. The Game receiver and
`stack_hover_name` body are source-hash/order evidence only. These tests do not
prove on-screen refresh, live resource reload, actual recipe-cache invalidation,
remote delivery or the complete native package's type acceptance.

## Content default versus explicit stack component

The official content declaration has `Item.components`; `mod_content::Plan::items`
patches those values into the generated `items.json` defaults. This is not the
same as an explicit ItemStack component patch. The observed `ItemStack::new`
creates `extra: None`; `item_nbt::item_from_nbt` decodes only its supplied
components; the catalog's `custom_components` collection excludes the
`minecraft:` namespace. Creative's rich export parses explicit stack NBT and
its legacy ID-only route calls `ItemStack::new`.

Therefore an arbitrary `minecraft:item_name` in a content declaration is **not
claimed to reach every generated stack**. Standard `item.NS.PATH` translations
already reach bare stacks through Names. For nonstandard item-name keys, the
explicit patch path is what this bounded fixture can prove. A separate full
creation-path audit is required before claiming automatic default-component
propagation, and no blanket patch/materialization hook is enabled here.

## Execution boundary

`prepare_item_display_language_fixture.py` has separate preparation, explicit
`--resolve`, and verification operations. Preparation checks the clean official
origin/revision and exact source hashes, then creates only a new small fixture.
It performs no Cargo call. The separate resolve phase adapts the copied official
lock using offline Cargo metadata and rejects any nonofficial dependency
identity except the one local test package. It verifies that each of the three
native crates occurs once at its real official path. No cache identity is
spoofed and no `#[path]` copy substitutes for a native crate.

Metadata and Rust execution need a later serial CI/capacity grant. Use an external
target directory so the sealed fixture root stays source-only, continuous disk
floor checks, bounded own-process-group cleanup and immutable pre/post seals.
At the present local capacity this is not permission to compile dependencies or
create a fresh host stage. The precursor authoring CI success does not cover
this new native fixture.

The separate title-routing, absent container keys, `%d` formatting and recipe
search cache observations remain in `i18n-item-recipe-audit-20261106.md`. This
fixture does not silently translate or alter those consumers.
