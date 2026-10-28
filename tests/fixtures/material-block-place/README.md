# Material block placement fixture

This fixture registers a test-only `tconstruct:fake_storage_block` using an
iron-block template so the native placement hook can be exercised before the
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
reload. It does not check remote-client state, clone/loot or rendered pixels.
