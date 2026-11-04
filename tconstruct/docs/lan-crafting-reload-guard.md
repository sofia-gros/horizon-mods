# LAN crafting-table reload guard (candidate)

After a world recipe-table reload, an open vanilla LAN crafting table can retain
a previously computed result. Before a guest's next table click, this guard
recomputes the result with the current world recipes and reapplies the native
craft-preparation callback. When the result changed, it cancels the click,
clears an active drag, and sends the full authoritative menu. The guest must
click again after receiving the correction. An unchanged result proceeds
normally. The existing TConstruct Crafting Station handler runs first.

The guard is lazy: an idle open table can show an old preview until the next
click. The candidate's standalone mock exercises the actual guard body but does
not reproduce the complete host, packet processing or Mixins integration.
The fixed host, actual reload, other native mods, and LAN are not yet verified.

The fixed host calls its table reload from `/reload` and datapack changes. The
guard invalidation uses the existing return hooks for those two callers because
the Mixins planner rejects a hook on `reload_world_tables` itself due to an
unsupported macro. A scoped official plan and apply succeeded against the
complete source trees of the two target crates, using a reduced workspace
manifest and only these hooks. This is not a full-bundle plan or a host build;
actual host injection, type checking, and runtime behavior remain unverified.

A later diagnostic compiled the fixed-host `horizon-client` staged source with
`rustc` using the prior offline Cargo dependency metadata and `opt-level=0`.
This succeeded with the guard callsite, open/close/leave hooks, reload and pack
RETURN wrappers, and generation invalidation body present. The diagnostic
stage also contains an unrelated one-line Station UI type-inference workaround;
it is a scoped, adjusted host probe, not an unmodified full-bundle Cargo build.
Actual reload behavior, integrated game execution and LAN remain unverified.
