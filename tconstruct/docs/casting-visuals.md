# Casting visual checks

Horizontal faucet models attach on the side opposite the outlet. The north
model's attachment is at z=16 and its outlet is near the centre; the other
facings use the blockstate rotations. The fluid stream now follows that model
rather than appearing on the opposite side. A downward faucet is unchanged.

Regression coverage checks all four horizontal stream bounds. Visual testing
in game is still needed for each facing, including the downward faucet, and
for both seared and scorched variants. This geometry correction does not
change fluid transfer or recipes.

Basin items use `none`, the fallback for the source's custom basin display
context, rather than the table's `fixed` transform. Custom model overrides
are not yet reproduced. This avoids applying item-frame scaling to
basin outputs. The source basin uses 12/16 size for output and 11.95/16 for
input; the current shared 12/16 scale differs slightly for input.

Fluid geometry raises block light to the source fluid's luminosity while
preserving sky light and any brighter world block light. Unknown fluids retain
world lighting. This affects all machine fluid geometry, not gameplay or world
lighting. It follows Mantle's rendering rule and the source TinkerFluids
registrations, but does not establish that lighting was the only cause of the
reported dark appearance. Visual verification remains necessary for molten
copper and other fluids in both bright and dark surroundings, plus basin
outputs with differing item models. Cooling fade remains separate work.

## Block lighting and table geometry

The source Station and Part Builder have a top and four legs, and the source
Basin is hollow; all three use non-occluding block properties. Preparation
now installs the source table collision/selection shape and marks these three
states as having zero light dampening, instead of inheriting a full-cube
stand-in's light blocking. Their existing non-occluding render flags remain.
The adjacent full-stone control is intentionally unchanged. Machine-shape
unit tests cover table legs and the Basin opening; the game fixture checks the
prepared flags and stone control, but has not yet run against real prepared
resources or verified screenshots.

The cache fingerprint changes so the prepared resources are rebuilt. This
alone does not prove that light already saved in existing world chunks will be
recomputed. Check freshly placed blocks and reloaded existing chunks in both
sky and block light. If old chunks remain dark, an explicit safe relight may
be needed; do not present a visual fix as verified until those checks pass.
