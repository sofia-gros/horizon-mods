# Automatic product-pair routing audit (unfinished, source-only)

Target: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and
Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. No upstream/fork edits or
access, no installed automatic hooks, no changes to the staged-pair runtime or
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
must stage allocation honestly rather than copy that call as an atomic test. Standard map postprocessing, output allocation,
remaining-item merge/drop rules and native event order must remain explicit.

## Next evidence gates

Read-only actual-source planner probes are proposed for Menu's owned-state
constructor, the local click_inventory craft call, and Crafter's result local.
The latter is bound by `let Some((recipe, mut result))`, while official Mixins
local-capture documentation rejects destructured bindings; verify the actual
error rather than assuming a workaround. Any successful plan is structural
only, not an ownership or consumption proof. Runtime and combined-package
validation must follow a bounded accepted design before production activation.

The companion JSON contains exact official file/function hashes and source
assertions. `tools/audit_crafting_product_routes.py` requires the clean pinned
official checkout and can reproduce them without changing external state.
