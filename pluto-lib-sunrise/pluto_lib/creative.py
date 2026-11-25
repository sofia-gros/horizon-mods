"""Register mod tabs in Horizon's prepared creative inventory data."""
import copy


def register_item_groups(inventory, groups):
    """Return a new inventory document with the registered item groups.

    Each group supplies an id, title, icon and ordered SNBT item stacks. Row and
    column use Horizon's existing Creative screen coordinates.
    """
    result = copy.deepcopy(inventory)
    existing = result['tabs']
    occupied = {(tab.get('page', 0) if tab.get('column', 0) < 5 else None,
                 tab.get('row'), tab.get('column')) for tab in existing
                if tab.get('type') != 'HOTBAR' and tab.get('visible', True)}
    ids = {tab.get('id') for tab in existing}
    for tab in groups:
        tab = copy.deepcopy(tab)
        row, column = tab['row'], tab['column']
        page = tab.get('page', 0)
        if not isinstance(page, int) or isinstance(page, bool) or not 0 <= page <= 0xffffffff:
            raise ValueError('Creative tab page is invalid')
        if tab.pop('auto_place', False):
            # Preserve already allocated tabs and find a free paged category slot.
            slot = max(page, 1) * 10
            while True:
                page, index = divmod(slot, 10)
                if page > 0xffffffff:
                    raise ValueError('No Creative tab position remains')
                row, column = ('TOP' if index < 5 else 'BOTTOM'), index % 5
                if (page, row, column) not in occupied:
                    break
                slot += 1
            tab.update(page=page, row=row, column=column)
        if row not in ('TOP', 'BOTTOM') or not isinstance(column, int) or not 0 <= column <= 6:
            raise ValueError('Creative tab position is outside the existing screen')
        position = (page if column < 5 else None, row, column)
        if position in occupied or tab['id'] in ids:
            raise ValueError('Creative tab position or ID already in use')
        if not all(isinstance(tab[key], str) and tab[key] for key in ('id', 'title', 'icon')):
            raise ValueError('Creative tab needs an ID, title and icon')
        if not isinstance(tab['items'], list) or not all(isinstance(item, str) for item in tab['items']):
            raise ValueError('Creative tab items must be SNBT strings')
        existing.append({**tab, 'type': 'CATEGORY', 'visible': True})
        occupied.add(position)
        ids.add(tab['id'])
    return result
