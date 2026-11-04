# Materials and You

This in-progress guide uses the original English Materials and You data and images from TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Craft it from a book and a pattern. The source-config-controlled starting-book advancement uses saved per-player progress.

Open the original cover and turn two-page spreads with the arrows or book controls. Chapter and page indexes link to the source text, illustrations, recipes and generated material and tool pages. Recipes cycle through loaded tag candidates and show their result count. Material pages show the part, its stats and its traits. A LAN guest requests the host-resolved pages. The Basic Armor section appears when its source item tag is populated.

The Tools chapter now includes the source's General, Ranged and Information group labels as navigable pages in its index. General and Ranged are shown when their respective source tags load at least one tool; Information leads into repair, part swapping, material swapping and recycling. This is a navigation adaptation of the source grouping, not a claim that every documented tool behavior is implemented. Representative General, Ranged and Information navigation and history were verified in a Linux creative fixture; a LAN guest also opened General and returned. Empty-tag behavior was source-checked but not exercised in game.

The following differences from Mantle's original reader remain:

- Illustrated material panels use the source colors, part icons, stats, traits and available material actions. Long material stat groups continue onto additional pages; exact parity with the original panel and every material is not established. Some tool properties and long image descriptions continue onto later pages, and long index entries use shortened excerpts.
- Tool and part preview hover text uses the existing item tooltip renderer. The earlier tested tree supports source modifier showcase tooltips. Exact page-element placement and image scaling have not been verified for every source page.
- The Modifiers introduction now links the five source entries (Traits, Upgrades, Slotless Modifiers, Abilities and Defense Modifiers) using their actual paginated destinations. The five links and history navigation were verified in a Linux creative fixture; a LAN guest also opened the introduction and followed Upgrades. This does not establish the implementation of every modifier described.
- Recipe diagram inputs and outputs use the existing item tooltip renderer on hover, including the displayed tag candidate and the damage shown in the repair example. Representative input, output and repair tooltips were verified in a Linux creative fixture, including changing tag candidates; a LAN guest also saw input and output tooltips. Not every recipe was exercised.
- Material actions requiring unregistered fluid buckets or instrument-specific item components are omitted rather than represented with an incorrect item. Their source display remains incomplete.
- Part Chest and Tinker's Chest have registered blocks and recipes; earlier representative checks are described in [chest details](tinker-part-chests.md). The Tinker's Chest dye recipe remains available, but the placed block does not display its dyed color. The guide now notes that limitation on its chest recipe page. Other source text may describe features that this port does not yet implement; current-main guide layout and LAN display remain unverified.
- Other languages and the remaining guides are future work.

An initial English Puny Smelting reader uses the same source revision and existing reader. Its original book-and-grout recipe and introductory, Melter, Moving Fluids, Casting, Worktable and Further Reading text and images are included. Worktable and Further Reading are source-data additions checked by automated tests, not yet checked in game. The Worktable recipe diagram depends on the source recipe and resolved tags being present in the host snapshot. The Puny Armor chapter uses its own source tag and descriptions and is omitted when that tag is empty. The ten built-in entries follow source order; additional datapack entries are sorted by ID because the host snapshot does not retain tag order. Armor page rendering and full preview fidelity remain unverified. The sand-cast diagram supports its source alternatives and output counts; a combined Linux build verified representative sand and red-sand cycling locally and on a LAN guest. The Tier 2 ranged materials chapter selects visible materials with ranged stats that are not in the distant material tag. Its current material panels reuse the existing reader and have not been checked in game. The armor materials chapter selects visible tier 0–2 materials with armor stats that are not in the distant tag. Its panels split the supported armor stat types across pages; order is a deterministic adaptation and exact source parity and in-game rendering remain unverified. The complete generated book has not been measured against the LAN transfer limits (512 pages and 262,144 bytes). If either limit is exceeded, the host sends no book response and the guest retains its locally generated pages, which can differ from the host. The Tier 2 melee and harvest materials chapter uses the source tier, non-distant predicate and the Whitestone exception when the whitestone ingots item tag is populated; it reuses the existing panel and is not game-verified. The Slotless chapter includes its four source overview pages and Damage Tools illustration, but its individual modifier pages, tag-driven contents, recipes and source layout are not yet implemented. The Upgrades chapter similarly includes its five static source overview pages. Both chapters still lack tag-generated individual modifiers and recipe displays; other source page formats may be incomplete, and the source describes equipment and features that may not be implemented. The combined build also verified supplied book-and-grout crafting, cover and chapter links, representative illustrations and LAN guest Melter display. The omitted chapters, unregistered icons and configuration loading remain incomplete or unverified. The source data and art are covered by `third_party/tconstruct/LICENSE`.

Mighty Smelting now has its registered book item, source cover and English introductory pages through the existing local and LAN reader. Its source casting recipe uses a book and 250 mB of seared stone; acquisition has not been tested in the integrated game. The three Tier 3 material chapters select visible, non-distant materials with compatible stats using the current world snapshot and reuse the existing material panels. Empty chapters are omitted. The book warns that listing source data does not establish that every material, trait or acquisition route is implemented. Material ordering is a deterministic adaptation, and the new chapters, panel rendering and LAN transfer size remain unverified in game. A separately labelled partial Smeltery Basics section shows the original Controller, Alloying, Seared Chute, Seared Ladder and Seared Glass pages. The notice warns that structures, fuels, alloys, controller casts and chute automation are limited, entity melting is incomplete, and connected glass faces are unavailable. Other Smeltery pages, entity melting, broad-tool listings, ability and other chapters are not implemented. The new pages and original controller image have not been verified in game or over LAN. A separately labelled partial Tool Advice section includes the original Tool Tips and Designing Tools text; it visibly notes that the broad-tool list, tank page and material tip icon are unavailable, and that tool features may be incomplete. The Anvil recipe diagram depends on the source recipe renderer. The introduction, cover, LAN synchronization, casting acquisition and full book parity are unverified in game.

The source recipe consumes one book after 250 mB has filled a casting table and 89 cooling ticks have elapsed. The table now refuses to begin or complete a cast when an occupied input slot does not contain exactly one item, protecting externally inserted or changed stacks from whole-stack loss. Existing recipe revalidation and occupied-output checks still apply. Hopper transfers now target only the input on insertion and the output on extraction, move at most one item per transfer, and require an empty fluid tank; a pending recipe also blocks transfers conservatively. Oversized inputs from other paths must be reduced before casting. If an external insertion creates an oversized input during a pour, interacting with the table returns the excess and retains one item and the pending fluid; this recovery path is not game-verified. Source-recipe lookup and saved-state transitions are tested, but the integrated acquisition path and hopper behavior have not been verified in game. Transfers fail closed when live slots disagree with saved metadata or a component cannot survive the saved-item codec; third-party inventory providers and in-game callback behavior remain unverified.

New LAN guide requests carry a sequence number, and the client accepts a reply only when both the sequence and guide identity match the open screen. This prevents a delayed reply from replacing a later opening, including a reopening of the same guide. New hosts still answer legacy one-byte requests; an older host will not understand the new request, so a newer guest retains its locally generated pages. These compatibility and race paths have unit tests but have not been verified in a running LAN session.

The source permits book images without explicit dimensions. The reader falls back to atlas dimensions and preparation resizes oversized guide images for the host's 1024×512 GUI sprite limit, keeping the pinned originals unchanged. Missing sprites no longer hide their accompanying text. Representative Melter, Casting, Moving Fluids and Channel illustrations were verified in the combined Linux build.

The modifier chapter overviews do not resolve `tinkering/tags/modifiers` or render the tag-generated modifier pages. The current pack snapshot captures material tags, not modifier tags, and does not expose a complete modifier registry against which required entries can be validated. Completing those pages needs ordered tag merging and registry resolution, modifier page content and translations, and modifier recipe selection with tool filters and the source's up-to-five-input display. A fixed built-in list would not honor datapack replacement, nested tags, optional entries or other registered modifiers. Those paths and the complete book's LAN transfer size remain unverified.

Puny Smelting showcase pages accept both source string and object item IDs. Gauge, Fluid Cannon and Casting Tank remain unregistered in this snapshot, so their icons may be missing until the items are available. Their source descriptions remain readable. A Linux creative fixture substituted stone for the unavailable source items and verified string/object forms and tooltip display locally and for a LAN guest; it did not verify the original icons or machine behavior.

The locally tested runtime corresponds to Git tree `f43c48bf04556085fa16967fdba1eee222ebbb8e` over commit `87d61cb5141511adbc5e8fe158e5cf4a16ccd692`. Linux representative checks covered a reset starting-book reward and reload without duplication, book-and-pattern crafting, cover, links on both pages, history back, a recipe link and tag-candidate cycling, source illustrations and tool previews, and long text with bold and italic styling. The source modifier icon tooltip was checked on hover. A temporary datapack populated the armor tag to inspect Traveler's Goggles; a separate host-only fixture verified that the guest received the extra armor chapter. Host and guest also showed the same styled page. The fixtures were removed after testing. These are not exhaustive checks of every page or platform.

The later local runtime tree `7eef3221b95cae12875527d7b94c1ea26806e567` passed the official Mixins transformation (revision `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`) and the Linux target build. In the existing test world, the Wood panel showed its source color, head/handle/binding stats and Cultivated trait; the wooden part and Broad Axe hover names appeared. Wrapped values stayed with their labels after a layout fix. The same Wood spread was visible to the LAN host and guest, and cover, both chapter links, history back and representative long further-reading text were checked. The Puny Smelting recipe showed its registered inputs and the expected blank unregistered result. These checks use representative pages, not every material or model. The new test screenshots are local only and have not been published.

The following figures are from the earlier two-page runtime `87d61cb`, not the new local tree.

![The original cover in the test world](materials-and-you-cover-test.png)
![A source crafting diagram in the test world](materials-and-you-recipe-test.png)

The crafting-grid texture is from Mantle 1.20 revision `17c287ad52f7b10154e07fa7570dc7bccd74ca2b`, under the MIT license in `third_party/mantle/LICENSE`.

The pinned guide JSON and images are ordinary files under `content/assets/tconstruct/book/`. The content overlay supplies the same paths to the reader and LAN host; resource preparation copies the original guide images into the GUI atlas. There is no separate guide ZIP to install. The `.hmod` content package and `.hnmod` native package remain separate, and this directory change has not yet been checked in a packaged game.

## Modifier book data groundwork

The package includes 59 pinned source modifier-tag files and 308 dynamic
modifier JSON definitions. On authoritative recipe reload, their raw resource
layers are captured in pack order; an enumerated file that cannot be read
makes that snapshot unavailable rather than silently incomplete. The snapshot
is cleared with the world and invalidated in parked dimensions on reload.
A tag resolver requires a complete, authoritative modifier registry and keeps
missing required entries and cycles as errors. A filename is not proof that a
modifier is registered: source static registrations, conditional definitions
and redirects still need a faithful registry implementation. No complete
registry or modifier pages are exposed by this groundwork. Manual original
pack directories retain their existing precedence and may lack these files.
Datapack reload, multiplayer synchronization, full Mixins integration and
actual book display remain unverified.

An isolated registry-composition helper now models the source's static-first
lookup and its single redirect pass over static and ordinary dynamic entries.
It requires an authoritative complete static ID set and an authoritative
classification of every highest-priority dynamic definition. The current host
provides neither: conditions, modifier deserialization and registration events
are not yet faithfully reproduced. The helper therefore has no production
consumer, and it must not be mistaken for a complete registry. Modifier tag
entries can resolve redirects to the target modifier object, so the existing
string-only tag resolver is not sufficient for displaying those entries.

A separate classifier models the source's order for unconditional redirects,
then the normal condition, then modifier deserialization. It requires the
caller to provide authoritative condition and deserializer results and returns
an error on unknown results. Conditional redirects are deliberately rejected:
the pinned source parses their conditions from the enclosing redirect object,
which needs further compatibility work. This helper is also not connected to
production and does not make the registry or modifier pages complete.

An unconnected tag helper can project IDs to canonical modifier identities
when supplied a complete authoritative registry. It preserves first-seen
ordering, deduplicates aliases, and treats an alias to the empty sentinel as
missing, as the source's tag lookup does. To avoid incorrect alias removal,
it currently rejects the snapshot if any modifier tag contains a remove
operation. It does not construct the registry or enable modifier pages. The
pinned TConstruct registration class alone cannot prove a complete registry:
other mods may register static modifiers through the registration event.

The bounded classifier now recognizes exactly two source-supported,
module-free shapes (the pinned Lustrous and Overslime Friend definitions),
unconditional redirects, and literal Forge true/false conditions. Any other
serializer or condition remains unknown, and one unknown prevents a complete
classification map. The two exact module-free shapes may also carry the literal
true condition, which the source loader ignores after evaluating. The full
pinned dataset contains many module-bearing
modifiers, so this is not a complete registry and is not connected to display.
Before production use, duplicate JSON object keys need rejection or verified
compatibility with the source parser; serde_json currently uses the last key.

Mighty Smelting's partial Smeltery chapter now includes four port-adapted text pages: features, a small structure example, controller operation and fluid-container operation. The chapter section names and order still follow pinned TConstruct `f4e77d43`, but these four page bodies are not verbatim source: the original diagram describes a different GUI, the original height limit differs from the current validator, and entity melting and some fluid-container paths remain incomplete. The source structure viewer, original GUI illustration, and other excluded pages are not implemented by this change. These pages and their layout have not been checked in game or over LAN.
