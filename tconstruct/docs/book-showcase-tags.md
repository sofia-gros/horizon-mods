# Item-tag showcase icons

The reader now resolves a tag-only source showcase (`{"tag":"namespace:path"}`)
from the current world's resolved item tags and known-item registry when the
book is built. It displays registered, valid, non-air members in deterministic
lexical order and cycles the icon every three seconds. Hover uses the exact
stack drawn for that frame. Single-item showcases and Creative Slot source
icons keep their existing paths. Mighty Smelting's Seared Blocks page uses its
original item-tag selector again.

This is a bounded implementation: empty, unavailable, malformed or unsupported
selectors show the existing text-only page rather than inventing tag members.
Tag selectors with NBT, counts, actions, arrays or extra fields are unsupported.
More than 128 valid members or over 4096 bytes of choice strings also falls
back to text, keeping generated metadata within the existing book wire budget.
The current icon dimensions are retained. The cycle uses a shared clock phase,
not the source's per-element timer or registry order. Already-open pages are
snapshots: reopen after a datapack reload. This does not implement click actions,
source error-item tooltips, arbitrary ingredient arrays or full book parity.

Rust tests exercise world isolation, replacement/missing/invalid tags, known
item filtering, size bounds, three-second selection/wrap and server book-reply
round-trip. Wire tests are codec-level, not real LAN gameplay. Static wiring
checks cover body pagination and matching item tooltip selection. Actual
rendering, hover, reload while open, save/reload and LAN remain unverified.

Source behavior: Mantle `17c287ad52f7b10154e07fa7570dc7bccd74ca2b`,
`ContentShowcase`, `IngredientData` and `ItemElement` (three-second switch and
matching current-item tooltip). See `../third_party/book-showcase-tags-NOTICE.md`.
