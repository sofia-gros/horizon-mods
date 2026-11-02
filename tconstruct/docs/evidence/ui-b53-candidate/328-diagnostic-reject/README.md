# Diagnostic Melter incompatible fluid rejection

Same generated-source diagnostic client, patch SHA, limited base and corrections as [the extraction fixture](../328-diagnostic-can/README.md); **not** an official Mixins application or full exact-head build of #328.

A copied fixture had 900 mB molten copper. A Copper Can was supplied with `fluid:"minecraft:water"` and right-clicked on the Melter output gauge. After the click, the can remained in inventory with its water component; after orderly shutdown the saved Melter still contained 900 mB molten copper. The screenshots are actual client captures and the script/log record setup and result. This is bounded local incompatible-fluid retention, not full invalid-input or exact-head coverage.
