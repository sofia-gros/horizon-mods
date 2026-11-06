# Item, menu and recipe translation compatibility

Read-only audit of official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`
and the frozen mod source `69c6b72b0b79b64317483bede6a2b2a689d5b83a`.
No implementation, resource file, locale state or production hook is changed.
No native/runtime test was run. Exact source hashes and caller lists are in the
[companion record](i18n-item-recipe-audit-20261106.json).

The requested author experience is that a mod supplies translation keys/data,
and PlutoLib handles selection-aware display, including item names. This audit
covers the actual item/menu/recipe paths. Language settings/reload and resource
format/parser ownership are separate work, not reimplemented here.

## Existing display paths to reuse

- `creative::Names::display_item` resolves the current language's
  `item.<namespace>.<path>`, then `block.<namespace>.<path>`, then the pinned
  English Names map. The last fallback is the raw path with underscores changed
  to spaces. Namespace defaults to `minecraft` for an unqualified item ID.
  The code uses the path as supplied, so do not silently replace slashes with
  dots or normalize arbitrary key punctuation.
- `Names::display_get` uses the current language, then its English map, returning
  None when neither contains the key. This differs from `lang::get`, which
  returns a missing key verbatim, and `lang::get_or`, which uses an explicit
  fallback. A wrapper must keep those distinct contracts.
- `Game::stack_hover_name` checks custom name, written-book title, item-name
  component, ominous-banner name, potion-specific name and player-head title
  before the default name. A shared item-display helper must preserve that
  precedence rather than replacing every name with an ID-derived key.
- `ItemStack::item_name` resolves `minecraft:item_name` through native component
  text in the current language. Its retained component stays translatable in
  saved/network data. This is different from storing an already translated
  String back into the stack.
- The content `Item` declaration has id, template, limits, rarity, components
  and remainder, not an independent `translation_key` field. Standard item keys
  need no new mapping registry. A supplied translatable item-name component is
  a candidate for an arbitrary key, but declaration-default propagation through
  every creation/tooltip/search path remains to be tested before promising it.

Recipe buttons/narration already derive names from the shown full ItemStack and
`Names::display_item`; the recipe-book screen receives a translation callback
for its native GUI keys. The new ingredient-presentation staging API must keep
full stacks, including their text components, and must not pretranslate or
rewrite them when selecting ingredient alternatives.

Existing menu examples use `container.inventory`, `gui.tconstruct.part_chest`,
`gui.tconstruct.tinkers_chest`, and native `gui.recipebook.*`/`itemGroup.search`
keys. Their author-facing wrapper can accept a key and fallback, but it cannot
assume every caller presently uses the same missing-key behavior.

## Native formatting is not Rust formatting

`horizon-client-core::text::Text::translate` and `translate_with_fallback`
resolve native templates and preserve their `translate`, `with` and optional
`fallback` source component. `format_translation` supports `%s`, numbered
`%n$s`, and `%%`. Unsupported conversions or missing arguments produce the
whole literal template; plain text arguments and styled Text arguments are
separate existing entry points. Preserve style/source component behavior and
native failure semantics rather than replacing them with Rust `format!`, ICU
or an invented brace formatter.

The frozen TConstruct English catalog has 3,628 keys, including 459 item keys,
555 block keys and 80 GUI keys. Nine values contain `%d` forms, for example
`item.tconstruct.cast.cost = "Material cost: %d"`. Native `%s` formatting will
not substitute those values. This is a concrete compatibility requirement to
resolve at the authoring-format boundary; the audit does not assert that all
nine keys are currently rendered or that an existing caller handles them.
TOML can be an authoring input only under a separately defined conversion
contract. It must not silently change the meaning of existing JSON strings.

## Existing consumer hazards before automatic localization

**Visible text is also being used as menu identity.** The frozen caller list
contains comparisons of `remote_menu_title().plain()` against English text such
as `TConstruct Crafting Station`, `TConstruct Part Builder Double`, Smeltery,
Foundry, Modifier Worktable and Melter. These checks affect client slot/data
handling and clicks, not just labels. Translating that title indiscriminately
can stop functional routing. The exact paths/lines are retained in the record.
A localized display and the stable protocol/menu identity must be separated
before changing those consumers. This is not permission to move actor or slot
policy into an i18n library.

**Some requested keys are absent.** `source-smeltery-draw.rs` looks up
`container.tconstruct.smeltery` and `container.tconstruct.foundry`, but neither
exists in this catalog. The corresponding existing keys are
`gui.tconstruct.smeltery` and `gui.tconstruct.foundry`. The present code therefore
uses English literals. A general resolver alone cannot repair this mismatch;
consumer correction must be explicit and scoped, not a global guessed alias.

**Recipe search has a separate cache obligation.** In official
`game/recipe_book.rs:402–440`, the view hash includes inventory, selected slots,
craft size, known-recipe version, filter, tab, search text and book kind, but no
language identity. Its `build_view` search reads current-language tooltips.
`load_language` explicitly forgets Creative items but does not visibly invalidate
this recipe view. A complete UI transition may invalidate it through another
path, so this is a source-derived test requirement rather than a confirmed
runtime failure. Verify that an existing nonempty recipe search refreshes after
a language change. Retaining a translatable source component also does not by
itself prove that every already-cached Text span is rebuilt automatically.

## Two-layer design implications and required evidence

The author layer can expose typed keys, translatable messages and item-display
requests. The internal layer should be the one official language/text adapter,
using native selected-language state and native item-name precedence. No second
CURRENT, cached locale, copied registry or host-dependent bundled API crate is
introduced by this proposal. Native host paths can remain in that thin adapter,
while the supported consumer-facing contract stays stable.

Before claiming automatic item display, verify at least: default item/block key
fallback; missing key versus explicit fallback; custom/book/item-name/potion
precedence; selected-language change; translated versus literal component save
and remote representation; styled/numbered arguments and malformed templates;
recipe button/ghost/tooltip names; and the English-title identity hazards.
Resource import/namespace validation, JSON/TOML collision policy and actual
reload/cache invalidation must join those proofs through their designated
owners. None of these runtime cases is marked passed by this source audit.
