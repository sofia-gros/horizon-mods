# Fluid Cannon supply through formed Drains

Issue #379, stacked on #816 / #747 / #740 / #736. A placed Seared Fluid Cannon
can supply a formed Smeltery or Foundry through its Drain and a Faucet. This is
fluid extraction, not Cannon firing or item automation.

The fixed source FluidCannonBlockEntity extends TankBlockEntity and delegates
non-item capabilities to that parent, which exposes its fluid tank on every
face. The native adapter uses the Cannon's existing 2,000 mB tank:
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/FluidCannonBlockEntity.java
- https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/component/TankBlockEntity.java

The source requires canonical saved state and an identical live item. Retained
wire-backed item data is refused. Only the nested tank is patched; the Cannon's
item is neither consumed nor rewritten in the live inventory. Saved/live source
state is checked again before the staged source/Faucet/controller commit.
Comparator and changed notifications follow successful saves.

The existing guarded Drain route preserves unrelated controller fields and
fluid layers, checks structure ownership and detected capacity, honors exact
fluid/tag identity and total free capacity, and pours immediately on first and
later reservations. A reserved packet retains its identity after a source refill.
Untagged fluid and registered vanilla base potions are supported. Custom potion
colors/effects remain unsupported even locally. Noncanonical or unreadable
state, missing entities, unloaded chunks, changed snapshots and failed saves wait.

Four fixture cases plus wiring coverage were added for both controller types,
item/metadata/layer preservation, packet timing and committed comparator changes,
seven failure states, potion identity after refill, the last 5 mB of capacity,
and local/network custom-potion refusal. Tests were not run. Review was a brief
static pass plus diff whitespace check. No CI, version checks or merge.

Other Cannon output destinations, generic capabilities, Duct input/filtering,
Cannon firing, redstone scheduling, actual host execution, save/reload and LAN
remain separate or unverified. The fixture mocks world, structure and storage.
