# Guide content and client observations

PlutoLib offers typed Rust entry points with explicit startup registration. A
consumer supplies its own namespace, content and policy; no TConstruct identifiers
or page layout are built in. This is an additive convenience layer, not a new
initialization framework or a second event registry.

## Register a read-only observer

In a mod attached to the shared client library, import the one installed API as
`horizon_client::pluto_inventory`. A bundled API example uses the equivalent
crate path `pluto_inventory_api`:

```rust
use horizon_client::pluto_inventory::client_payload::{self, Payload, RegistrationError};

fn init() -> Result<(), RegistrationError> {
    client_payload::OBSERVED
        .channel("example:guide_document", 4096)
        .register("example:guide_logger", log_document)
}

fn log_document(payload: Payload<'_>) {
    // Inspect untrusted bytes synchronously. Do not infer sender identity,
    // reply authority or ownership of any screen from this callback.
    println!("{} bytes on {}", payload.bytes.len(), payload.channel);
}
```

Call `init` once from your consumer's existing startup path. This API does not
invent a native ready event or automatically install an input handler. The
installed adapter observes after native packet observation; calling the public
`observe` adapter entry yourself does not prove that ordering. Observation grants
no send, cancel, reply, screen-update or identity capability.

The previous spelling remains supported and uses exactly the same registry:

```rust
client_payload::register("example:guide_logger", "example:guide_document", 4096, log_document)?;
```

Choose one spelling for a given subscriber. Registering the same ID through either
entry rejects the duplicate rather than replacing it. `OBSERVED` is zero-sized;
its copies refer to the same installed service. `channel` borrows its settings and
does not allocate, validate or register. Validation and owned selectors are created
by `register`, in the same ID/channel/limit error order as the old function.

The explicit limit is 0 through 262,144 bytes; zero observes only empty payloads.
The borrowed bytes are not cloned. IDs determine lexical dispatch order. One
subscriber snapshot is taken before invoking callbacks, outside the registry
lock; registration changes made during a callback affect the next dispatch.
Callbacks are function pointers or non-capturing closures, must return promptly,
and must not panic. This is trusted in-process code, not a sandbox.

Remove a subscriber explicitly when your consumer no longer needs it:

```rust
client_payload::OBSERVED.remove("example:guide_logger")?;
```

The old `client_payload::remove` is identical. Neither a subscriber ID nor the
borrowed channel settings is an ownership token or an RAII subscription handle.
Dropping the settings does not remove a registered callback.

## Build bounded content without native internals

```rust
use horizon_client::pluto_inventory::guide::{Document, Span, Style};

let document = Document::new(vec![
    vec![Span::plain("Example guide"), Span::styled("Next page", Style {
        page: Some(2), ..Style::default()
    })],
    vec![Span::plain("Any mod can supply its own content")],
], |_| false)?;
```

`guide::{Document, Page, Span, Style, RequestTracker}` re-exports the existing
types; all old module paths remain valid. The Span constructors only create
values. `Document::new` still checks page/span/text/wire limits, one-based link
targets and the caller's explicit metadata policy. The example rejects metadata;
another consumer may admit its own bounded opaque values. There is no silent
normalization or new action execution.

See the complete [standalone example](../mixins/pluto_native_creative/inventory-api/examples/guide_observer.rs).
It uses the actual shared API, registers explicitly, builds a document
and removes its subscriber. It does not send a packet or construct a Game/window.

## What this does not add

- No guide-open, screen-ready, request-send or reply-apply event is claimed.
- RequestTracker remains explicitly restarted in-process correlation, initially
  inactive. It does not track a world, actor, connection or opening automatically.
- The current-book capability remains a separately guarded, synchronous native
  operation, not a public durable session or a way to refresh an overlaid guide.
- The existing payload ceiling is unchanged. A legacy TConstruct reply envelope
  can be 262,152 bytes, so this observer is not a drop-in LAN response replacement.
- No native hook, package source guard, registration order or existing LAN feature
  changes. The old TConstruct response hook conflict remains unresolved.

The actual Font12 proof at fixture `59c0651b` stays immutable and is separate from
the API proof below. Neither proof implies new installed/native/UI/LAN coverage. See
[the remaining LAN/lifetime boundaries](guide-lan-safe-connection-status.md) and
[the original observation contract](client-payload-observation.md).

## Validation of the public API source

Five source regressions pass, including exact inverse reconstruction of the old
registry/dispatch and Document implementation. At exact CI head
`2ee73a700647a5a73142cd20fea065b44c8de598`, [run 42176619324](https://github.com/sofia-gros/horizon-mods/actions/runs/42176619324)
passed all six added Rust unit tests inside the 143-test inventory suite. The
actual `guide_observer` example compiled and its one named test passed separately.
The final source, lock, file-set and generated-output seals matched. Guide input
was the unchanged published `39499dc85ab4489b251a0cf0216bb37299f667ae`.
The Guide tests cover cross-API
registration/removal, validation precedence, shared lexical order and borrowed
bytes, reentrant registration, preserved document validation and inactive request
state. These are API contracts, not native observer or live GUI behavior.
