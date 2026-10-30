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
