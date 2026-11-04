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
