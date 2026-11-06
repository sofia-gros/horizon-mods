# Keep a fluid's content and render definition together

**Additive API candidate, not yet compiled or executed.** Based on render source
`510f0fa89551f89c81097052ced06235037a6451` / published `a432ce4561ae1c81448b37a5b19e71126d8c4e72`.
This is a later cutoff than both verified ordinary v15 and the frozen render CI.
It does not inherit native/CPU/GPU results from either. No consumer is activated.

The existing seven-field Definition and register_all API are already concise.
A fluent builder would not remove meaningful work unless it invented flow
policy defaults. This change instead removes repeated Definition/render pairs
across content generation, optional material selection and explicit registration.
It helps an author reuse the same chosen values. No migrated real consumer or
observed mismatch bug is claimed as evidence.

`fluid::Declaration::new(definition, render)` validates both existing values and
keeps private fields with read-only accessors. It does not register anything.
`content()` and `render_material_model()` delegate to the existing helpers.
`fluid::register_declarations(entries)` delegates once to the existing atomic
`register_all_with_render` operation. There is no second registry, revision,
bootstrap, event bus or new flow/render behavior.

```rust
use pluto_api::fluid::{self, Declaration, Definition, RenderDefinition, RenderOrientation};

fn rainwater() -> Result<Declaration, String> {
    Declaration::new(
        Definition {
            id: "meadow:rainwater".into(),
            block: "meadow:rainwater_block".into(),
            bucket: "meadow:rainwater_bucket".into(),
            texture: "meadow:block/rainwater_still".into(),
            tick_delay: 5,
            drop_off: 1,
            light: 0,
        },
        RenderDefinition {
            flowing_texture: "meadow:block/rainwater_flow".into(),
            tint: 0xFFFFFF,
            orientation: RenderOrientation::Bottom,
        },
    )
}

// Consumer build/preparation tooling can use the same declaration:
// let declared = rainwater()?;
// let files = declared.content()?;
// let optional_model = declared.render_material_model()?;
// The consumer installs files and declares optional_model, not PlutoLib.

// In approved startup wiring, before resource/model preparation:
fn initialize_fluids() -> Result<(), String> {
    fluid::register_declarations([rainwater()?])
}
```

The two preparation results are data, not evidence that content was installed.
Generation may happen in separate build tooling; registration still belongs to
actual startup. The native standalone-model boundary precedes native_blocks on
the selected target. Callers must preserve that ordering and the relevant side;
this convenience value does not wire hooks or manufacture loaded resources.
No Result is silently ignored and no water-like flow policy is supplied by default.

## Explicit style semantics are unchanged

Every Declaration includes explicit render facts. Repeating an identical pair
is idempotent. An explicit default style conflicts with an existing custom style,
just as register_all_with_render does. This differs from legacy register(def),
which preserves an already registered custom style. Do not use a default
Declaration as a semantic replacement for that legacy repeat. The full batch
still rejects conflicts without publishing earlier new entries; allocation
panics are not a recoverable transaction or rollback promise.

Definition and RenderDefinition struct literals and all existing functions
remain available. The accessors borrow the owned values; cloning a Declaration
copies data, not a native identity/authority token. Top orientation remains
render geometry only. The fluid contact query stays read-only, and a generic
fluid-contact event remains unimplemented. No Game mutation/cure authority is
hidden behind this registration helper.

## Validation scope

Six new tests in the real API module cover invalid construction, unchanged
content/model delegation, default content equivalence, shared registry lookup
and idempotence, explicit-style conflict rollback versus legacy repeat, and an
empty batch. They are prepared but unrun until a bounded native test slot is
granted. Small source checks separately verify the unchanged original fluid
body, private fields, direct delegation and recursive source packaging. Those
checks are not Rust type checks, installed-package proof or gameplay results.

## Two-layer maintenance boundary

The author-facing layer is the owned Declaration value and the batch function.
The underlying definition validation, content generation and registration
transaction stay the single implementation. Horizon-sensitive adapters remain
behind PlutoLib's existing internal boundary, not in this value or consumer
callbacks. This is an additive public convenience: existing entry points and
behavior are unchanged, and host upgrades still require the normal internal
adapter/complete-plan checks. It makes no permanent compatibility promise.

Source checks: **4/4 passed in 0.008 seconds**. Six Rust tests remain unrun;
no Cargo, native planner/apply, host build, runtime or new CI job was executed.

The source guard was then made independent of local-only Git objects: it checks
fixed original-body and packager SHA256 values, so a fresh published checkout
can run it. Its first local attempt reported a missing packager input in the
sparse checkout (three checks passed, one setup error). After including that
unchanged tracked file, the portable checks passed **4/4 in 0.001 seconds**.
The declaration implementation and six unrun Rust tests did not change.
