# Retained armor break feedback

Partial high-priority #379 armor durability integration, stacked on #937. The
ordinary retained wear path already marked broken armor and refreshed equipment,
but did not notify the wearer that a piece had broken.

## Behavior

After a unique, unchanged stack receives the final committed durability loss,
recognize an intact-to-broken transition for its verified stock armor slot.
Produce the native equipment event 49/50/51/52 for head/chest/legs/feet exactly
once. An already broken item, shield-only spend, zero or partial loss, replaced
item, wrong slot or removed stack produces no event. Existing callback pairing
also rejects missing or duplicate calculations before this stage.

Keep the broken stack equipped, including its materials, upgrades and saved
shield state. The presentation path does not invoke native
`player_equipment_broke`, because that method also raises PlayerDestroyItem.
No item is destroyed and no destroy-item callback or statistic is manufactured.

For the acting guest, enqueue the existing native `guest_equipment_breaks`
event. `as_guest` broadcasts it to the guest and observers and restores the
outer queue. Native remote handling reads the corresponding equipment slot to
render feedback; it does not remove the item. In a window simulating the world,
use the existing stack-aware break sound and five item particles at the acting
player. Dedicated servers do not render or consume presentation RNG. Host-local
feedback follows the existing native local route; this adds no new host-avatar
broadcast protocol.

The pre-break item snapshot supplies local sound/model identity. Current native
model resolution and component patches remain authoritative. Both Rust and
Python resource preparation add the ordinary item-break sound default for armor
when absent, preserving an existing default. Regenerate prepared item tables to
pick this up. The native sound helper continues to respect per-stack overrides
or removal; no fallback sound is forced on a stack that removes its component.

All pieces are committed first, then feedback is emitted before the existing
equipment/Maintained refresh. Existing per-event deferred-writeback and native
batched health ordering are unchanged. Presentation uses native RNG, not a claim
of source client/server random-stream identity.

## Limits and review

Ordinary supported worn armor only. Held weapons/shields, modifier-caused wear,
custom break animations/models and the remaining unsupported modifier
combinations are outside this change. Actual audiovisual behavior, event packet
ordering and dedicated/guest/LAN remain unverified. This does not close the
complete #379 equipment row.

No native extension or provenance-blocked fluid code is included. The base
protection chain still requires native 09f0179db and 5e47547e2.

Source behavior: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT),
ModifiableArmorItem.damageItem and ToolDamageUtil: invoke the break callback
when damage breaks a tool, then return zero to suppress vanilla item depletion.
Native boundary inspected: Game::player_equipment_broke, as_guest queue handling,
remote_equipment_break and item-particle/sound helpers.

Brief static API/borrow/order and whitespace review. Two regression cases added
but not run, covering all four slot events and silent nonbreak cases. Build,
planner and tests belong to separate batch integration. No CI inspection or
merge was performed here.
