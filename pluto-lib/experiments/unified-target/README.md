# Candidate native integration

This is a planning record for Horizon `cabe8b5de7d0b45541c923c4a6b3c5cb3e3686c7` and Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. It is not a supported combination. See [compatibility](../../docs/compatibility.md) for the combinations actually built and run.

The candidate stages `pluto_native_creative`, `pluto_transport_3a`, `pluto_tool_rules`, `pluto_component_display_3a`, `pluto_component_bar`, and `pluto_items_b2`. The staged manifest revisions were changed only in the disposable test directory. The transport source also included the unmerged shape-normalizer at commit `b57d342177f49b36c7ac4d33d236f6c2dd201b67`; this helper does not migrate old saves. Do not install the old Creative paging adapter or old item network adapters alongside these bundles.

An official Mixins plan succeeded with these six bundles (9 injections, 4 modules, 2 crates). A second plan with the existing Tool rules and wand demos also succeeded (13 injections, 6 modules, 2 crates). The demo's optional Creative cancellation probe must remain disabled for normal use. Plans check transformation structure, not Rust compilation or game behavior. Existing pure fixtures were run with cached `b2e71...` dependencies; the candidate host itself has not been built.

## Imports today

| Task | Current entry |
| --- | --- |
| Build Creative tabs | `pluto_creative_api::{register_tabs, Entry, Tab}` |
| Prepare or normalize Tool SNBT | `horizon_client::pluto_tool_codec::{prepare_tool, normalize_tool_shape}`; the latter exists only in the unmerged candidate |
| Evaluate Tool rules | `horizon_items::items::pluto_tool_rules::{parse_tool, Tool, Rule, Outcome, evaluate}` |
| Register short item behavior | `pluto_item_api_b2::{register, ItemBehavior, ItemData, UseContext, UseOutcome}` |
| Standard component tooltip and bar | Stage the display and bar bundles; no consumer import is needed |

These are the current staged paths, not a completed stable facade. In particular, the item crate name contains a host suffix and Tool APIs are exposed inside host crate paths. A future compatibility-preserving public entry must hide those paths before consumers can rely on identical imports across target revisions. No broad rename is proposed in this planning unit.

## Ordering and verification

At `native_tooltip`, the wand demo registers at priority 200, the component tooltip runs at 100, then the item tooltip runs at 100 (mod ID order). At `use_tool`, wand registration runs at 200 and the item adapter at 100. At the stack encoder, the max-damage and Tool RETURN hooks run in that order, each retaining an already present or removed component. The standard component bar is proposed at priority 1000 before ordinary `native_item_bar` dispatch; another Mixins injection can still override or be overridden according to its priority. A plan establishes ordering, not semantic compatibility with arbitrary mods.

For the next authorized combined build, use one disposable overlay/world and verify both demos together: Creative registration, pages and search; accepted Tool and max-damage transport, save/reload and tooltip/bar; Tool speed/drop; and wand local/LAN short-use Pass, Handled and Cancel. Compare vanilla bucket Pass behavior because the host changed bucket handling, and test the changed Creative stack-count boundary without claiming atomic cancellation. Check the separate demo items and other namespaces survive. Tinkers integration, Windows and full launcher installation remain separate checks. Capture a real combined-demo screen for any PR.
