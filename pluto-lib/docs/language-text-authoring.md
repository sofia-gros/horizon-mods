# Translation keys at the display boundary

This is an additive source candidate, separate from the successful Guide authoring CI at `2ee73a7`. The native language adapter and this author-facing layer must be composed and verified together before this entry point is available in an installed package. No consumer or existing LAN handler is changed here.

## Intended author API

```rust
use horizon_client::pluto_language::{key, Key, Text};

const TITLE: Key<'static> = key("gui.example.guide.title").with_fallback("Field Guide");
const COUNT: Key<'static> = key("gui.example.guide.count").with_fallback("Pages: %s");

// Call from the consumer's existing admitted draw/rebuild path.
fn labels(page_count: usize) -> (Text, Text) {
    (TITLE.resolve(&[]), COUNT.resolve(&[Text::literal(page_count.to_string())]))
}
```

The same public imports are used in the [standalone example](../examples/language_text.rs).

`Key` borrows the translation key and optional fallback. Construction allocates nothing and does not register a callback. Each `resolve` delegates once to the existing `pluto_language::text` or `text_or` adapter, which delegates to the official native Text constructor. There is no second language registry, locale cache or formatter. Native missing keys, English fallback, indexed placeholders, styles, malformed-format behavior and component serialization remain native behavior. Translation keys are not resource IDs, and this layer adds no new key syntax restriction.

The returned value is the same native `Text`, not a wrapper or a live reference to the key. Its spans reflect the language at resolution time. Keep a `Key` when later text should use a later selected language; resolve it again only when the consumer is allowed to rebuild its display. Already-resolved argument Text values are not recursively translated again. Resolve those arguments too when that is the intended behavior.

This layer is trusted synchronous mod code, not an input size limiter or a sanitizer for arbitrary server data. Native Text may carry styles/components outside the bounded Guide Document codec; obtaining Text does not bypass that codec's admission rules. It does not grant permission to interpret an action or replace an open screen.

## Integration contract

The shared owner installs `language.rs` as the client-library `pluto_language` module. This candidate supplies `language-text.rs` as the sibling `pluto_language_text` module and publicly re-exports `key`, `Key` and the identical native `Text` from `pluto_language`. Avoid importing `Text` twice if the internal adapter already imports it. Both modules remain in the same host library; no bundled-crate reverse dependency or registry is added.

The shared adapter's exact source/commit, installed module entries and re-export must be sealed with this source before any native proof. The existing pure authoring CI input, Guide Document implementation, consumer navigation, packet observation guard and ordinary package manifest remain unchanged in this checkpoint.

## Verification boundaries

- Three Python source guards check the one-call delegation, borrowed configuration and test inventory. They are not Rust compilation.
- Six native tests are supplied in the helper and are **not run yet**. They require actual official core `Language`, `ResourceSource`, thread-scoped lookup and Text, plus the actual shared `language.rs` adapter. The fixture must not replace those methods with mocks. They cover retained-key re-resolution versus old Text snapshots, missing keys/fallback components, English fallback, styled reordered arguments, malformed placeholders and a shorter borrowed fallback.
- A fixture should also compile the exact public re-export/example above, not just private helper names. Full installed module composition still needs its own official plan/apply/type gate.
- No renderer, glyph atlas, bidirectional visual layout, open-book lifetime, pack reload UI action or LAN response test is included. `lang::scoped` is suitable for isolated text lookup tests but does not change the process-global bidirectional setting.

## Existing UI connections remain explicit

The API can serve draw-time labels like the current builder/station/smeltery lookups. It does not automatically migrate those consumers. Retained Guide pages still need a safe owned rebuild path, source-book locale selection and compatible pagination/history handling. Book/wire and remote machine identities currently depend on English titles; translation must not silently alter those identifiers. The existing LAN handler still generates literal pages without selecting the guest locale.

See [the screen-by-screen source audit](guide-i18n-boundaries.md). No localization event, guest locale ownership or retained-screen reply authority is implied by this concise authoring syntax.
