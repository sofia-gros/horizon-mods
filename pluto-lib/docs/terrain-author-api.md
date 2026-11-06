# Two layers for client terrain rendering

This additive source checkpoint is prepared, not native-compiled. It is separate
from the frozen material CI source and does not inherit that run's results.
No TConstruct renderer is registered or activated by this change.

Mod authors use `horizon_client::pluto_render::terrain`. The native renderer
implementation keeps its existing `mesher::pluto_terrain_*` modules. The facade
reexports their actual types and forwards registration to their one registry;
it adds no registry, hook, initialization event, asset owner or dependency edge.
Old public paths and registration methods remain valid.

## Explicit client startup

```rust
use horizon_client::pluto_render::terrain::{
    CELLS, MaterialContext, PaintedBatch, RegisterError,
};

fn init_client_renderers() -> Result<(), RegisterError> {
    CELLS.register("example:slurry", render_slurry)
}

fn render_slurry(_cell: &MaterialContext<'_>) -> Option<PaintedBatch> {
    // Returning None deliberately keeps the native ordinary block model.
    // Replace this with the mod's pure, bounded cell geometry policy only
    // after its content and native material preparation are installed.
    None
}
```

The mod invokes its startup function explicitly. This example demonstrates the
public registration path only; it does not create a fluid or draw custom cells.
`CELLS.register_all(&[(group, callback), ...])` registers a material-aware batch
atomically. `register_geometry` and `register_geometry_all` retain the old,
explicit geometry-only mode. Material-aware callbacks require explicit prepared
appearance data. Default styles produce no such data, so geometry-only is the
compatible mode for them. Geometry-only callbacks cannot silently ignore
explicit appearance data. Do not mix ownership of the same group across modes.

`CELLS` is zero-sized. It is not an ownership token or RAII subscription. Copies
refer to the same existing process-wide startup registry. There is no removal,
replacement or thaw operation. First mesh access freezes registration, including
when the registry is empty. Existing duplicate/error ordering and 256-group /
256-byte-name limits apply. A callback may serve many groups; each group still
counts. Nothing raises the 12-quad cell or section output budgets.

## Preparation and policy remain separate

Common content declarations remain in `pluto_api::fluid`; they are not stored in
this client facade. Register Definition and RenderDefinition together, install
the generated assets, and declare any standalone material model before official
resource assembly. On official 624c, `native_standalone_models` precedes
`native_blocks`, so late registration in native_blocks alone cannot prepare the
secondary model. This facade does not insert an earlier callback for you.

The prepared catalog retains its existing exact Assets/RenderAssets/MeshTables
owner triple and registry-revision checks. Missing/unavailable/stale preparation
and unsupported native face metadata retain the existing native fallback. The
facade does not reinterpret a resource-generation number as ownership or a
callback return as permission to mutate/render outside the staged operation.

`build` and `build_oriented` are the original geometry functions. In a renderer
callback use cell-local origin `[0, 0, 0]`; their general section-relative origin
support is not admitted by the cell adapter. `PaintedBatch` chooses only prepared
Primary/Secondary material facts, not arbitrary atlas layers or pass flags.
Top anchoring changes geometry only, not simulation/contact. Callbacks must be
pure, bounded and safe on meshing workers. No per-cell wrapper allocation, lock,
resource load, callback interception or output conversion is added here.

## Verification boundary

The source guard reconstructs the previous rendering.rs byte-for-byte by removing
this one appended facade, checks the same manifest and lower implementation
hashes, and checks exact forwarding bodies. Two Rust tests are prepared for type
identity and invalid-registration behavior, but are not executed at this cutoff.
They do not publish entries into the global registry. No installed native type,
loader, CPU mesher differential/performance, gameplay or GPU claim follows from
these source checks or the startup example.
