# pluto lib

pluto lib is a shared library for Horizon mods. Its first feature adds mod tabs to the prepared Creative inventory. Item, block, armor, effect, GUI and inventory APIs are planned for later.

## Add a Creative tab

A mod supplies an ordered list of item stacks in Horizon's SNBT format. This example adds one item to an unused tab position:

```python
from pluto_lib.creative import add_tabs

updated = add_tabs(prepared_inventory, [{
    'id': 'example:tools',
    'title': 'Example Tools',
    'icon': '{count:1,id:"example:hammer"}',
    'row': 'TOP',
    'column': 5,
    'items': ['{count:1,id:"example:hammer"}'],
}])
```

`add_tabs` returns a copy; it does not edit the input. IDs, icons and stacks must refer to items registered by the mod. A stack may include components in its SNBT. The current Horizon screen provides a fixed set of tab positions; choose an unused one. The function rejects duplicate IDs or positions.

Tinkers' Construct uses its own tab data in `tconstruct/content/creative-tabs.json`. This resource path and the Creative screen have not yet been verified together in a game build.
