"""Select source-compatible material identities for Creative tool variants."""

# ToolPartItem stat types from the pinned TConstruct tool-parts registration.
_HEAD = {'pick_head', 'hammer_head', 'small_axe_head', 'broad_axe_head',
         'small_blade', 'broad_blade', 'adze_head', 'large_plate'}
_PART_STATS = {**{name: 'head' for name in _HEAD},
    'bow_limb': 'limb', 'bow_grip': 'grip', 'bowstring': 'bowstring',
    'arrow_head': 'arrow_head', 'arrow_shaft': 'arrow_shaft', 'fletching': 'fletching',
    'tool_binding': 'binding', 'tough_binding': 'binding',
    'tool_handle': 'handle', 'tough_handle': 'handle', 'maille': 'maille',
    'shield_core': 'shield_core', 'ribcage': 'ribcage', 'shell': 'shell', 'laces': 'laces',
    'helmet_plating': 'plating_helmet', 'chestplate_plating': 'plating_chestplate',
    'leggings_plating': 'plating_leggings', 'boots_plating': 'plating_boots'}


def required_stats(definition):
    """Return material stat IDs in source part order, or None for no materials."""
    for module in definition['modules']:
        if module['type'] == 'tconstruct:part_stats':
            parts = (part if isinstance(part, str) else part['item'] for part in module['parts'])
            return ['tconstruct:' + _PART_STATS[part.removeprefix('tconstruct:')] for part in parts]
        if module['type'] == 'tconstruct:material_stats':
            return [stat if isinstance(stat, str) else stat['stat'] for stat in module['stat_types']]
    return None


def single_material_variants(requirements, materials):
    """Select each visible material, falling back to the first with each stat.

    ``materials`` is source-ordered (id, stat-ID set). A material produces a
    variant only if at least one requirement accepts it, matching the source rule.
    """
    fallback = {kind: next((name for name, stats in materials if kind in stats), None)
                for kind in requirements}
    variants = []
    for name, stats in materials:
        if not any(kind in stats for kind in requirements):
            continue
        selected = [name if kind in stats else fallback[kind] for kind in requirements]
        if all(selected):
            variants.append(selected)
    return variants


def build_variants(tabs, definitions, materials):
    """Expand tool stacks using pinned definitions and visible material stats.

    Each generated stack carries the selected source identities. Material-derived
    tool stats and traits are not synthesized here.
    """
    import copy
    import json
    import zipfile

    with zipfile.ZipFile(materials) as archive:
        visible = []
        prefix = 'data/tconstruct/tinkering/materials/'
        for path in archive.namelist():
            if not path.startswith(prefix + 'definition/') or not path.endswith('.json'):
                continue
            material = path.removeprefix(prefix + 'definition/').removesuffix('.json')
            definition = json.loads(archive.read(path))
            stat_path = prefix + 'stats/' + material + '.json'
            if definition.get('hidden', False) or stat_path not in archive.namelist():
                continue
            stats = set(json.loads(archive.read(stat_path)).get('stats', {}))
            visible.append((definition.get('tier', 0), definition.get('sortOrder', 0),
                            'tconstruct:' + material, stats))
        visible.sort(key=lambda row: (row[0], row[1], row[2]))
        ordered = [(name, stats) for _, _, name, stats in visible]
    result = copy.deepcopy(tabs)
    with zipfile.ZipFile(definitions) as archive:
        for tab in result:
            expanded = []
            for raw in tab['items']:
                # The source tab file contains only simple, registered item IDs.
                name = raw.split('id:"', 1)[1].split('"', 1)[0]
                short = name.split(':', 1)[1]
                definition_name = 'slime_chestplate' if short == 'slimy_chestplate' else short
                path = 'data/tconstruct/tinkering/tool_definitions/' + definition_name + '.json'
                if path not in archive.namelist():
                    expanded.append(raw)
                    continue
                required = required_stats(json.loads(archive.read(path)))
                if required is None:
                    expanded.append(raw)
                    continue
                for identities in single_material_variants(required, ordered):
                    expanded.append(json.dumps({'count': 1, 'id': name,
                        'components': {'minecraft:custom_data': {'tic_materials': identities}}}, separators=(',', ':')))
            tab['items'] = expanded
    return result
