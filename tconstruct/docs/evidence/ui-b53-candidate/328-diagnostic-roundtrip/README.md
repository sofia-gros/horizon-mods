# Diagnostic local and guest Copper Can round trips

Same generated-source diagnostic client, patch SHA, limited base and corrections as [the extraction fixture](../328-diagnostic-can/README.md); **not** an official Mixins application or full exact-head build of #328.

From a copied world with 900 mB molten copper and no ongoing melting, the local player filled a Copper Can by clicking the Melter output gauge, then right-clicked the gauge with it to empty it back. A LAN guest repeated the same pair. Logs show both filled cans containing `fluid:"tconstruct:molten_copper"` and both emptied cans without that component. The host save after orderly shutdown contained 900 mB. Screenshots are real client captures. Fuel gauge, invalid containers, unrelated screens and formal exact-head integration remain separately gated.
