# Direct Melter container interaction

Use a supported filled container on a Seared Melter to add fluid, or an empty
container to take fluid out. A successful transfer happens before the normal
menu opens. If no transfer is possible, the existing menu-opening path runs.
Sneaking while holding an item keeps the existing item-use behavior.

The ordering follows pinned Tinkers' Construct
[`TinyMultiblockControllerBlock.use`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/controller/TinyMultiblockControllerBlock.java):
try `FluidTransferHelper.interactWithTank`, then call the controller use method.
This includes attempting fluid transfer before any formed-structure GUI check.

This implementation reuses the existing supported container transactions and
the Melter's 2,160 mB single-fluid capacity. It stages one container, preserves
all machine fields except the changed tank contents, preserves tank extension
fields, and saves before changing the held stack or giving/dropping a result.
Insufficient room or fluid, incompatible fluid metadata, unsupported items and
unreadable state do not consume the held item. Creative mode keeps the held
container; draining additionally gives the filled result, following the existing
portable tank interaction policy. No fuel or melting input is consumed.

## Verification boundaries

The native-dependency container fixture tests bucket insertion/extraction,
stack remainder, capacity/mismatch/registry/opaque-input rejection, separation
of tagged fluids, and preservation of machine/tank fields. Python checks the
interaction ordering and failed-save guard. Run:

```
HORIZON_HOST=/path/to/pinned/Horizon bash tconstruct/tests/run_seared_reinforcement_rust.sh
python3 -m unittest discover -s tconstruct/tests -p 'test_melter_held_container.py'
```

This is limited to the shared container implementation's supported forms;
arbitrary external fluid capabilities and unsupported potion metadata are not
added. Menu formation/status behavior is unchanged. Actual world interaction,
creative inventory edge cases, save/reload, offhand and LAN remain unverified.

Draft PR #631 at `75531f4bd2f45d4a569ef923a61ca13631d5d95d` changes store writes to
staged `write_many`. Its additional live inventory/projection validator is
conditional on `tconstruct:smeltery`, not Melter; the Melter retains the existing
metadata-ID and chunk checks. This interaction uses that same store entrypoint
and returns before inventory mutation on rejection. Its files are not modified or
merged here. This is a source compatibility audit, not a combined host/runtime
integration test with that draft.
