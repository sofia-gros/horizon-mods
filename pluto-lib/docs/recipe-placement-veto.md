# Veto-only automatic recipe placement

This boundary controls native item-ID automatic placement. It neither implements
component-aware allocation nor establishes persistent ownership of a recipe.
TConstruct's material marker, recipe family, preview selection and manual
crafting rules remain consumer policy. No production consumer is activated.

## Public policy and ordering

The shared `recipe_book::placement` API has one process-lifetime registry.
Register during startup before books are built. Each callback gets read-only
requested/resolved IDs, definition/local/remote phase, recipe form, optional
display facts and borrowed ingredient views. Cell holes, custom kind/JSON,
accepted IDs, display order and nested ingredient shape remain explicit.
Special definition ingredients are currently unavailable, not silently empty.
The library never interprets a custom kind or component key.

A callback returns Pass or Deny. Pass is not an authorization and cannot reverse
another denial or a native rejection. Callbacks run outside the lock in lexical
registration order. Any denial wins; Silent dominates an optional IfLocal
preview request. Duplicate/empty registration IDs are rejected without replacing
an existing callback. Callbacks must be read-only and deterministic for the
supplied facts, without actor state or a retained world/recipe cache.

The canonical definition gate returns native None on denial before the original
`Recipes::placement` body. It follows that body's cooking-first lookup and exact
name normalization, without querying RecipeIndex. An unknown recipe retains
native behavior. With no provider it continues without constructing facts.

The UI attempt gate runs before `Game::place_recipe` sends a remote packet or
opens its local menu operation. On denial it may explicitly request the existing
local preview capability, but its result is ignored for authorization: missing
book, wrong menu, stale selection, invalid display or geometry cannot reopen
placement. This is a best-effort presentation request, not an Allow decision.
No provider means the original native path runs unchanged. The outer click
handler may already have updated last-placed/ghost presentation before reaching
this gate; the veto does not roll back those ordinary UI changes. It protects
automatic slot allocation, not every preceding presentation event.

## Official paths and limits

On official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0:

- Ordinary and native-mod book clicks reach Game::place_recipe.
- Local Game::place_recipe consults Recipes::placement before menu placement.
- Menu::place_recipe repeats that query before testing/clearing the grid,
  counting ingredients, placing slots or recomputing the result.
- LAN placement checks actor authority, display-to-recipe lookup and knowledge,
  then queries Recipes::placement before either guest menu path.
- Book::build uses Recipes::placement for its craftable projection.

The fixture preparation pins these source paths and ordering assertions. The
menu/game fixture executes exact official function bodies with modeled outer
state, mutation and network endpoints. It is not a live LAN or GUI test.

Remote displays may omit the custom ingredient metadata of their source recipe.
A local callback cannot infer an unseen rule and may pass such a request to the
server. The authoritative server must register the corresponding definition
policy; installing the client-only preview service is not server protection.
Late registration also does not rebuild an existing book cache. Follow the
startup contract, or explicitly refresh through the separate invalidation API.

Manual ingredient testing and hand-arranged crafting/assembly do not go through
this allocation gate and are intentionally unchanged. The service does not
claim to control arbitrary inventory writes by other mods.

## Composition and migration

The two native target boundaries are conservatively exclusive in the complete
installed-plan guard. An earlier cancellable HEAD could bypass the callback;
a RETURN modifier could restore a denied placement. Other hook composition must
be proved explicitly before it is allowed. Consumers use the shared registry
rather than installing competing raw hooks at those targets. Their old hooks
must be removed when ported; installing both is not a supported migration.

A test-only TConstruct callback is kept under tconstruct/tests. Its shaped/custom
marker predicate is compared with the exact marked(Recipe) function from MOD
8c56bb52b6b77fae6115e894af9e369a5c3f9e08. It preserves top-level-only marker
recognition; nested or shapeless forms are not silently widened. Optional local
preview selection remains a separate family-prefix/nonempty-preview rule.
This comparison is not production startup wiring or certification of the whole
old branch. The library itself contains no TConstruct IDs or marker rules.
