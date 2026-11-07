# Pinned colored slime compression discovery

The three JSON files are copied unchanged from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, with `kind` equal to `sky`,
`ichor` or `ender`:

`src/generated/resources/data/tconstruct/advancements/recipes/redstone/common/slime/{kind}/slimeblock.json`

https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/advancements/recipes/redstone/common/slime

`sha256.json` records their exact bytes. Native adaptations map recipe IDs to
`{kind}_slime`, rename `recipe` to `recipes`, and use `items: "#tag"` instead of
legacy `tag`. Current-world matching additionally requires the shared bridge.

Copyright (c) SlimeKnights. MIT license at
`tconstruct/third_party/tconstruct/LICENSE`.
