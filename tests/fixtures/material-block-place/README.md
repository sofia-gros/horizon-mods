# Material block placement fixture

This fixture registers a test-only `tconstruct:fake_storage_block` using a
dirt template so the native placement hook can be exercised before the
original block is registered. Its appearance, properties and loot are not
TConstruct behavior. The fixture output must not be used as a release resource
pack or as evidence of original block rendering or networking.

From the repository root, with a Mixins-transformed game binary and an existing
base item resource overlay from `tools/resources.py`:

```sh
python3 tests/fixtures/material-block-place/prepare.py \
  --game target/build/fast/horizon-client --base target/item-resources \
  --output target/material-block-fixture
python3 tests/game/material_block_place.py \
  --game target/build/fast/horizon-client --ctl target/build/fast/horizon-ctl \
  --resources target/material-block-fixture
# Repeat with --in-frame for the other single-player execution mode.
```

Run under a working display. The test creates a new disposable world and checks
the original `Material` and block-entity ID bytes after placement, save and
reload, then checks Material on the normally mined drop and that reusing the position
with an untagged stack does not inherit the old material. It does not check
remote-client state, clone, explosion behavior or rendered pixels.

The fixture loot deliberately includes an extra diamond before the fake block
to exercise multiple drops; this is not original loot. When staging
`tests/fixtures/material-drop-proof` as an additional independent Mixin mod,
pass `--expect-proof` to verify that its `proof` component survives both on a
separate earlier diamond drop and on the fake-block drop, while the material
is attached to the correct fake block. The proof mod is a test fixture and is
not part of the release resources.

Pass `--reverse-drop-order` to the fixture preparation and `--reverse-order`
to the game test to check the other drop order. These switches affect only the
fixture, not released loot.
