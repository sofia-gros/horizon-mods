# Register a Creative tab

`pluto_lib.creative.register_item_groups` adds mod tabs to Horizon's prepared `creative-inventory.json`. It returns a new document and leaves the input unchanged.

Add `pluto-lib/` at the repository root to your Python module path, then pass the parsed inventory and a list of tabs:

```python
from pluto_lib.creative import register_item_groups

inventory = register_item_groups(inventory, [{
    'id': 'example:tools',
    'title': 'Example Tools',
    'icon': '{count:1,id:"example:hammer"}',
    'row': 'TOP',
    'column': 5,
    'items': ['{count:1,id:"example:hammer"}'],
}])
```

The item IDs must already be registered by your mod. `icon` and `items` are SNBT item stacks; item components can be included in a stack. Items appear in the supplied order. Choose a position not used by a visible tab: the current Horizon screen has seven columns in each row. Duplicate IDs and positions are rejected.

Tinkers' Construct supplies its tab data in `tconstruct/content/creative-tabs.json`. Its resource preparation adds the tab without changing the original Horizon resource file. A Linux game test selected this tab and obtained a material-bearing pickaxe and helmet. Other platforms remain unverified.

Fabric developers can think of each group definition as the ID, title, icon, and ordered entries used when registering a custom Creative tab. Unlike Fabric's runtime registry and `CreativeModeTabEvents.modifyOutputEvent`, this Python function updates a prepared resource document before the game starts; it does not register a runtime callback or change an existing tab. Register the items themselves separately.

For playing Tinkers' Construct, see its [player guide](../../tconstruct/docs/playing.md). For the shared library overview, see [pluto lib](../README.md).

![The Tinkers Creative tab selected in a Linux game](../../tconstruct/docs/pluto-creative-tab.png)

*The pickaxe tab opens the registered Tinkers equipment list.*
