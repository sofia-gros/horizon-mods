# Automatic product-pair routing audit (automatic integration blocked)

Target: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. No upstream edits or fork access, no installed automatic hooks, no changes to the staged-pair runtime or
the matching/placement vetoes. The source audit passes 22 pinned assertions;
that is source-order evidence, not gameplay or transaction validation.

## No single existing automatic boundary has been proved

A global assembly callback plus a remainder callback is insufficient. Even an
identical immutable input can produce different pairs if the callback is
stateful or random. Sealing registration does not prove purity. `find` is also
used for rendering, recipe availability, previews and retries, without a later
consumption at all. A cached lookup by recipe ID or input equality does not
establish which authoritative operation owns it.

There are four authority/operation families, plus nonauthoritative remote
prediction. They must not be silently combined into one presumed route:

1. Local normal result click: `Game::menu_ops` explicitly declines slot 200 for
   non-QuickMove. `Game::click_inventory` then has a separate slot-200 loop:
   `craft_recipe` -> derive remainders -> give/cursor publication -> decrement
   inputs -> apply remainders -> crafted events. It rejects remote execution
   earlier. A Menu-only port would miss this real path.
2. Local engine shift-click/other engine operations: `Menu::click` can invoke
   `quick_move` repeatedly, with `on_take`/recompute after each successful move.
   `Game::menu_session` copies values in and only publishes after a successful
   callback, but this one outer call may contain several crafts and maps/events.
3. LAN host inventory and table clicks: `guest_menu_op` copies the authenticated
   guest inventory into the owned Menu, runs the operation, copies it back and
   publishes events/drops. The table path also checks spectator/table validity.
   `native_prepare_item_craft` can replace the result after the operation in both
   inventory and table routes. A pending receipt cannot blindly trust that later
   result slot still belongs to its prepared pair.
4. Scheduled Crafter: one `crafter_dispense` call finds output with the world's
   map context, changes crafting state, postprocesses maps, derives remainders
   from the original input, ejects output and remainders, then decrements the
   block inventory. Those world effects are not the same as a Menu value commit.

A syntactic scan of production source finds exactly three remainder calls:
Game, Menu and Crafter. Menu is shared by several authority families, so three
call sites do not mean three interchangeable transactions.

Remote prediction remains separate. The Menu Env has recipes/knowledge but no
server-authority token. Installing a generic Menu hook does not by itself prove
that its caller is the authoritative local player or LAN host. The existing
local and remote routes must retain their separate packet/event behavior. The
remote route predicts through menu_session, then sends changed-slot hashes and
the server state in ContainerClick packets; it is not a second authoritative
product grant.

## Why on_take alone is too late

`safe_take` removes the result before `on_take`. The normal pickup path has
already set the cursor before calling it; quick_move has moved items into
inventory; swap has written the player slot. Carried-bundle insertion also
extracts through safe_take. Crafting result slots are excluded from pickup-all.
ResultContainer removes the entire output, even when a right-click requests a
smaller amount; do not import ordinary-slot splitting into this contract.

`on_take` itself calls find twice, then remainders, before consuming grid cells.
A cancellable hook here that rejects a stale pair would leave previously moved
output unless every earlier publication were rolled back. Returning early is
not an atomic denial. It also needs the same already-selected output/recipe,
not a newly computed callback result.

## Bounded candidate, not an implemented solution

A private owned receipt on the actual Menu instance is worth testing for
feasibility. Menu is nongeneric, has private fields, no derived Clone, and one
visible Self constructor in `Menu::new`. This is only a source observation;
Mixins must still accept the complete actual constructor body and type-check
all uses. The receipt would belong to that instance, not a pointer-keyed global
cache or an assumed permanent recipe index.

Even if state injection works, safety still requires all of the following:

- Generate/store the complete pair once, keeping its actual source and output
  snapshots and selected native recipe. Preserve native matching order,
  no-provider continuation and limited-crafting/knowledge policy.
- Before any result leaves the Menu, revalidate the complete input and actual
  output against the same receipt on try_remove, quick_move and swap routes.
  Handle the bundle extraction/toggle path honestly. A stale/overridden receipt
  must refuse before publication, not recompute unrelated remainders afterward.
- Consume that same receipt once, apply its remainders, then regenerate the next
  result after consumption. Test quick-move retries, full inventory, no movement,
  cancellations and repeated identical grids. A preview-only disposable Menu
  must not be treated as a live crafting transaction.
- Invalidate on native result overrides, grid changes, new Menu construction,
  recipe/provider changes and failed operations. Do not assume a long-lived Menu
  when local menu_session constructs one for a batch while LAN stores its Menu.
- Reserve the necessary complete-plan boundaries against earlier cancellation
  and later result resurrection. Existing matching exclusivity stays intact.

This Menu candidate still does not cover local direct result clicks or Crafter
world publication. Those need separate authority-owned derive-and-consume
adapters that carry one owned pair through their complete original operation.
Replacing broad native bodies without proof is not a small shared hook and is
not enabled by this audit. In particular, `Game::give` mutates inventory and
offhand and can return a leftover; it is not a pure allocation check. The legacy
slot-200 shift fallback checks that leftover before consuming inputs. This
audit has not proved that fallback reachable when the ordinary QuickMove route
is handled by Menu, so it does not claim a live duplication bug. A new adapter
must stage allocation honestly rather than copy that call as an atomic test.
Standard map postprocessing, output allocation,
remaining-item merge/drop rules and native event order must remain explicit.

## Actual official planner results

Three uninstalled, no-op feasibility probes ran against the pristine official
export using the exact pinned Mixins binary. All three were rejected by the
whole-body safety pass with `E_UNSUPPORTED_CONTROL_FLOW`:

- `Menu::new`: adding an inert private receipt state and its explicit `Self`
  constructor is rejected. The native constructor contains closures.
- `Game::click_inventory`: an interior `BEFORE_CALL` at `craft_recipe` is
  rejected. This cannot currently be treated as an accepted local pair boundary.
- `Game::crafter_dispense`: an interior `BEFORE_CALL` at `Menu::map_crafted`
  is rejected. The later local-capture resolution was not reached; the separate
  source rule rejecting destructured locals is not an observed diagnostic here.

Each terminal plan exit was 1, not a disk stop. No plan artifact was written,
no apply or compile followed, and no automatic hook was installed. Minimum free
space was 10,894,979,072 bytes, above the continuous 10 GiB +128 MiB floor.
The first launcher attempt omitted the existing toolchain environment, so all
three initially stopped at `E_METADATA` before source planning. Those logs are
retained separately; unchanged probe sources were rerun with the environment
loaded, producing the source-level rejections above. These setup failures are
not counted as unsupported native targets.

`crafting-products-route-probes-20261106.json` records exact source, binary,
manifest and log hashes, diagnostics and interpretation. The inert probes live
under `tests/probes/crafting-products-routes/` and are excluded from normal
PlutoLib/TConstruct packages. The source audit JSON remains reproducible by
`tools/audit_crafting_product_routes.py` without changing external state.

## Remaining boundary and evidence gates

The explicit pair-staging library is independently native-type-checked in the
complete v14 package. It remains useful to an explicit synchronous caller, but
its returned value is not authority to change a live grid or publish items.
Automatic integration remains blocked for the three exact candidates above;
this is not proof that every possible mod-only design is impossible.

A different route must first demonstrate one authority-owned operation carrying
the same pair through output allocation, input consumption, remainders and
native events. A broad cancellable HEAD adapter copying a whole native operation
would be a new implementation, not a proven workaround; it requires complete
preservation of all four authority families, remote prediction, native special
recipes, cancellation, quick-move retries and other mods' composition. Do not
silently replace rejected interior hooks with that larger scope. No such adapter
is enabled here, and no fork or official source modification is proposed.
