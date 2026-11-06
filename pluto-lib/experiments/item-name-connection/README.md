# Item-name connection feasibility probes

Status: prepared only. No plan, apply, Rust compilation or game execution has run.
These identity/no-op helpers are excluded from the normal PlutoLib package.
They test official extension-point binding feasibility, not functional behavior.

Official target: Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0 and
Mixins e8cd0aa38cffef49cb2e088ef716baa107148c02.

## Proposed contracts, not enabled behavior

- Receive: modify the result of the existing components_tag call in
  from_network and template_tag. Preserve an explicit item-name removal before
  native decode and before the top-level Wire baseline is constructed.
  Do not rewrite the larger components_tag function, whose nested closures use ?.
- Send: a recursive network_stack HEAD is a candidate for an explicit-removal
  adapter. The eventual implementation must prove exactly one native body
  evaluation per stack, nested container/projectile/bundle behavior, original
  input preservation, usable raw-Wire fast paths and canonical re-encoding
  separately. An identity entry plan proves none of those requirements.
- Defaults: wrap the existing Items::load -> Self::load_with call. A possible
  implementation would hold native item_defaults::shared(root) across that
  native call so both reads share one actual Arc snapshot, not a second global
  cache. How to retain the name data on the resulting Items is still a design
  decision; the probe stores nothing and changes no field. Headless catalog,
  reload, default-name removal, actual Game display and styled defaults are
  not covered.

No production package/manifest is edited. These no-op probes must not be
reported as production connection, a typed complete package or an executed
default/removal test. A plan can reject individual targets independently.
No function-copy replacement or user-fork access is proposed.
