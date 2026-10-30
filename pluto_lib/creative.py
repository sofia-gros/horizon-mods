"""Register mod tabs in Horizon's prepared creative inventory data."""
import copy


def add_tabs(inventory, tabs):
    """Return a new inventory document containing the ordered mod tabs.

    Each tab supplies id, title, icon and a list of SNBT item stacks. Row and
    column use Horizon's existing Creative screen coordinates.
    """
    result = copy.deepcopy(inventory)
    existing = result['tabs']
    occupied = {(tab.get('row'), tab.get('column')) for tab in existing
                if tab.get('type') != 'HOTBAR' and tab.get('visible', True)}
    ids = {tab.get('id') for tab in existing}
    for tab in tabs:
        row, column = tab['row'], tab['column']
        if row not in ('TOP', 'BOTTOM') or not isinstance(column, int) or not 0 <= column <= 6:
            raise ValueError('Creative tab position is outside the existing screen')
        if (row, column) in occupied or tab['id'] in ids:
            raise ValueError('Creative tab position or ID already in use')
        if not all(isinstance(tab[key], str) and tab[key] for key in ('id', 'title', 'icon')):
            raise ValueError('Creative tab needs an ID, title and icon')
        if not isinstance(tab['items'], list) or not all(isinstance(item, str) for item in tab['items']):
            raise ValueError('Creative tab items must be SNBT strings')
        existing.append({**tab, 'type': 'CATEGORY', 'visible': True})
        occupied.add((row, column))
        ids.add(tab['id'])
    return result
