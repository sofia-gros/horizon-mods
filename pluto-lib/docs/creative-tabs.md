# Register a Creative tab

`pluto_lib.creative.add_tabs` adds mod tabs to Horizon's prepared `creative-inventory.json`. It returns a new document and leaves the input unchanged.

Add `pluto-lib/` at the repository root to your Python module path, then pass the parsed inventory and a list of tabs:

```python
from pluto_lib.creative import add_tabs

inventory = add_tabs(inventory, [{
    'id': 'example:tools',
    'title': 'Example Tools',
    'icon': '{count:1,id:"example:hammer"}',
    'row': 'TOP',
    'column': 5,
    'items': ['{count:1,id:"example:hammer"}'],
}])
```

The item IDs must already be registered by your mod. `icon` and `items` are SNBT item stacks; item components can be included in a stack. Items appear in the supplied order. Choose a position not used by a visible tab: the current Horizon screen has seven columns in each row. Duplicate IDs and positions are rejected.

Tinkers' Construct supplies its tab data in `tconstruct/content/creative-tabs.json`. Its resource preparation adds the tab without changing the original Horizon resource file. The Creative screen integration is not yet verified in-game.

For playing Tinkers' Construct, see its [player guide](../../tconstruct/docs/playing.md). For the shared library overview, see [pluto lib](../README.md).
