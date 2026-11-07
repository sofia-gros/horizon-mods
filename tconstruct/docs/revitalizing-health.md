# Revitalizing: ordinary worn armor path

Addresses the high-priority #379 Jeweled Hide max-health/equipment-change gap.
Pinned source: TConstruct f4e77d43, modifier `revitalizing`, material
`jeweled_hide`, AttributeModule and UpdateHealthModule.

Jeweled Hide supplies Revitalizing through armor stats. Each level adds two
maximum health using source float coefficient evaluation widened to double.
Properly slotted, unbroken supported armor must carry the current worn tag.
Current material traits and exact canonical modifier definition are required;
unknown definitions, malformed levels and nonzero saved upgrades fail closed.

Equipment completion and published recipe/definition reload refresh the bonus.
Gold Guard and Revitalizing changes are projected together before health is
clamped to the final maximum, avoiding an intermediate health loss when changing
between the two traits. Equipping or repairing does not heal. Removal, breakage,
source invalidation and material changes remove or recompute the transient bonus.
The native attribute registry remains responsible for its normal bounds.

This is the normal armor trait path only. The source's separate held-tool
UpdateHealth bonus heals on equip and applies its custom damage on removal;
that path, command-added upgrades, remote prediction and LAN are not implemented
here. Ordinary Jeweled Hide binding/bowstring grants Fortune, not Revitalizing.
Source equipment event ordering across different simultaneous slot changes is
not claimed bit-identical. The adapter tests the final worn maximum and clamp.

The retained Rust tests exercise the production projection and shared refresh using real ItemStack/NBT and
a narrow Game/trait/attribute adapter. Full host compilation, actual gameplay,
save/load and network round trips remain separate gates. No persistent tracker
or serialized schema is introduced. The common game-private health-attributes module is added to Pluto; the existing TConstruct hooks and policy sources remain the consumers.

The PR source is adapted at the current integration, not copied with its inherited branch tree. The common helper publishes all native additive max-health modifiers before a single clamp; it never heals and rejects duplicate IDs or non-finite requested values before publication. Native setter failures are not rolled back. No new callback, registry or saved state is introduced.

Source checks are separate from execution. These Rust adapter tests and the actual Game host have not yet been compiled or run for this intake. Main inclusion and closing PR697 remain pending the required build and available error checks.
