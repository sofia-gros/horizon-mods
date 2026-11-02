# Registration API

`Registry::new(namespace)` accepts a lowercase content namespace other than `minecraft`. `item(name, template)` and `block(name, template)` declare names under it using vanilla template IDs. Names can contain slash-separated resource path segments. A block also creates a block item, so item and block names may not collide. `write_to` requires a new output directory and creates Horizon's content declaration files. It does not write the manifest, models, textures or translations.

`add_creative_tabs(&inventory, &tabs)` returns a copy of the JSON document. A tab has a namespaced ID, nonempty title and icon, `TOP` or `BOTTOM` row, column 0–6, and an ordered list of SNBT stacks. Duplicate IDs and occupied visible positions are rejected; Horizon's hotbar position may be reused, matching the existing Python helper. The API does not validate SNBT syntax or whether its item IDs are registered; supply valid stacks for registered items. The original inventory is never changed.

This is preparation-time registration, not Fabric's runtime registry or a runtime tab event. Template registration does not attach runtime behavior; the experimental [Tool rules adapter](tool-rules.md) is separate. Vanilla templates and available properties are determined by the target Horizon version. The demo's Creative overlay must be installed separately from its content mod.

For more tabs than the screen has positions, see the experimental [Creative paging API](creative-overflow.md). It requires the bundled Mixins adapter and uses the pinned Horizon item parser to populate search; unlike `add_creative_tabs`, it checks the supplied stack syntax.
