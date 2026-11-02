# Original guide inventory

Source: TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2. Reader coverage is static inspection, not runtime validation.

## materials_and_you

| Chapter | Source entries and page types | Reader coverage |
| --- | --- | --- |
| intro | introduction [mantle:text; intro/welcome.json; copied]<br>pattern_craft [mantle:crafting; intro/pattern_craft.json; copied]<br>partbuilder_craft [mantle:crafting; intro/partbuilder_craft.json; copied]<br>partbuilder [mantle:image_text; intro/partbuilder.json; copied]<br>tinkerstation [mantle:image_text; intro/tinkerstation.json; copied]<br>tinkerstation2 [mantle:text; intro/tinkerstation2.json; copied]<br>tinkerstation_craft [mantle:crafting; intro/tinkerstation_craft.json; copied]<br>storage [mantle:text; intro/storage.json; copied]<br>partchest_craft [mantle:crafting; intro/partchest_craft.json; copied]<br>tinkers_chest [mantle:crafting; intro/tinkerschest_craft.json; copied]<br>hidden [mantle:image_text; intro/selena.json; copied] | source page formats partially supported; runtime unverified |
| melee_harvest_materials | dynamic/no-load; {'type': 'tconstruct:melee_harvest', 'tier': {'min': 0, 'max': 1}, 'predicate': {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'}} | partial: dynamic filters/pagination and visuals require runtime confirmation |
| ranged_materials | dynamic/no-load; {'type': 'tconstruct:ranged', 'tier': {'min': 0, 'max': 1}, 'predicate': {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'}} | partial: dynamic filters/pagination and visuals require runtime confirmation |
| ammo_materials | dynamic/no-load; {'type': 'tconstruct:ammo', 'tag': 'tconstruct:book/basic_ammo'} | partial: dynamic filters/pagination and visuals require runtime confirmation |
| tools | group_general [mantle:text; no-load; no file]<br>group_ranged [mantle:text; no-load; no file]<br>group_info [mantle:text; no-load; no file]<br>repairing [mantle:crafting; tools/repairing.json; copied]<br>part_swapping [mantle:image_text; tools/part_swapping.json; copied]<br>material_swapping [mantle:image_text; tools/material_swapping.json; copied]<br>recycling [mantle:image_text; tools/recycling.json; copied] | source page formats partially supported; runtime unverified |
| armor | dynamic/no-load; {'tag': 'tconstruct:modifiable/book_armor/materials_and_you', 'path': 'armor', 'sort': False} | partial: dynamic filters/pagination and visuals require runtime confirmation |
| modifiers | index [mantle:index; modifiers/index.json; copied]<br>traits [mantle:text; modifiers/traits.json; copied]<br>upgrades [mantle:text; modifiers/upgrades.json; copied]<br>slotless [mantle:text; modifiers/slotless.json; copied]<br>abilities [mantle:text; modifiers/abilities.json; copied]<br>defense [mantle:text; modifiers/defense.json; copied] | source page formats partially supported; runtime unverified |
| further_reading | furtherreading [mantle:text; furtherreading/furtherreading.json; copied]<br>punysmelting_craft [mantle:crafting; furtherreading/puny_craft.json; copied]<br>tinkergadgetry_craft [mantle:crafting; furtherreading/gadget_craft.json; copied]<br>fantasticfoundry_craft [mantle:crafting; furtherreading/fantastic_foundry_craft.json; copied] | source page formats partially supported; runtime unverified |

## puny_smelting

| Chapter | Source entries and page types | Reader coverage |
| --- | --- | --- |
| intro | (blank) [mantle:blank; (blank); no file]<br>thruul [mantle:text; introduction/thruul.json; copied] | source page formats partially supported; runtime unverified |
| melter | melter [mantle:image_text; melting/melter.json; copied]<br>features [mantle:text; melting/features.json; copied]<br>meltergui [mantle:image_text; melting/meltergui.json; copied]<br>meltergui2 [mantle:text; melting/meltergui2.json; copied] | source page formats partially supported; runtime unverified |
| moving_fluids | thruul [mantle:image_text; movingfluids/thruul.json; copied]<br>moving_fluids [mantle:image_text; movingfluids/intro.json; copied]<br>faucets [mantle:image_text; movingfluids/faucet.json; copied]<br>faucets2 [mantle:text; movingfluids/faucet2.json; copied]<br>channels [mantle:image_text; movingfluids/channel.json; copied]<br>channels2 [mantle:text; movingfluids/channel2.json; copied]<br>copper_cans [mantle:text; movingfluids/copper_can.json; copied]<br>copper_cans_craft [mantle:crafting; movingfluids/copper_can_craft.json; copied]<br>gauge [mantle:showcase; movingfluids/gauge.json; copied]<br>fluid_cannon [mantle:showcase; movingfluids/fluid_cannon.json; copied] | source page formats partially supported; runtime unverified |
| casting | casting [mantle:image_text; casting/intro.json; copied]<br>sand_cast [mantle:text; casting/sandcast.json; copied]<br>sand_cast2 [mantle:image_text; casting/sandcast2.json; copied]<br>sand_cast_craft [mantle:crafting; casting/sandcast_craft.json; copied]<br>sand_cast3 [mantle:image_text; casting/sandcast3.json; copied]<br>gold_cast [mantle:image_text; casting/goldcast.json; copied]<br>other_cast [mantle:image_text; casting/othercast.json; copied]<br>automation [mantle:image_text; casting/automation.json; copied]<br>casting_tank [mantle:showcase; casting/casting_tank.json; copied] | source page formats partially supported; runtime unverified |
| melee_harvest_materials | dynamic/no-load; {'type': 'tconstruct:melee_harvest', 'tier': 2, 'predicate': {'type': 'mantle:or', 'predicates': [{'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'}, {'type': 'mantle:load_condition', 'conditions': [{'type': 'mantle:tag_filled', 'tag': 'tconstruct:whitestone_ingots'}], 'if_true': {'type': 'tconstruct:id', 'id': 'tconstruct:whitestone'}}]}} | not implemented |
| ranged_materials | dynamic/no-load; {'type': 'tconstruct:ranged', 'tier': 2, 'predicate': {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'}} | not implemented |
| armor_materials | dynamic/no-load; {'type': 'tconstruct:armor', 'tier': {'min': 0, 'max': 2}, 'predicate': {'type': 'mantle:inverted', 'inverted_type': 'mantle:tag', 'tag': 'tconstruct:book/distant'}} | not implemented |
| armor | dynamic/no-load; {'tag': 'tconstruct:modifiable/book_armor/puny_smelting', 'path': 'armor', 'sort': False} | not implemented |
| upgrades | group_general [mantle:text; upgrades/general.json; no file]<br>group_melee [mantle:text; upgrades/melee.json; no file]<br>group_damage [?; (blank); no file]<br>group_harvest [mantle:text; upgrades/harvest.json; no file]<br>group_ranged [mantle:text; upgrades/ranged.json; no file]<br>group_armor [mantle:text; upgrades/armor.json; no file]<br>group_armor.helmet [?; (blank); no file]<br>group_armor.chestplate [?; (blank); no file]<br>group_armor.leggings [?; (blank); no file]<br>group_armor.boots [?; (blank); no file] | not implemented |
| slotless | group_slotless [mantle:text; slotless/slotless.json; no file]<br>damage_tools [mantle:text_image; slotless/damage_tools.json; no file]<br>group_bonus_upgrades [mantle:text; slotless/bonus_upgrades.json; no file]<br>group_cosmetic [mantle:text; slotless/cosmetic.json; no file] | not implemented |
| worktable | index [mantle:index; worktable/index.json; no file]<br>modifier_worktable [mantle:crafting; worktable/modifier_worktable.json; no file]<br>invisible_ink [mantle:image_text; worktable/invisible_ink.json; no file]<br>visible_ink [mantle:image_text; worktable/visible_ink.json; no file]<br>sort_modifiers [mantle:image_text; worktable/sort_modifiers.json; no file]<br>remove_modifiers [mantle:image_text; worktable/remove_modifiers.json; no file]<br>extract_modifier [mantle:image_text; worktable/extract_modifier.json; no file]<br>extract_enchantment [mantle:image_text; worktable/extract_enchantment.json; no file]<br>extract_upgrade [mantle:image_text; worktable/extract_upgrade.json; no file]<br>extract_defense [mantle:image_text; worktable/extract_defense.json; no file]<br>extract_ability [mantle:image_text; worktable/extract_ability.json; no file]<br>toggle_interaction [mantle:image_text; worktable/toggle_interaction.json; no file]<br>(blank) [mantle:right_padding; no-load; no file] | not implemented |
| further_reading | mighty_smelting [mantle:image_text; further_reading/mighty_smelting.json; no file]<br>encyclopedia [mantle:image_text; further_reading/encyclopedia.json; no file] | not implemented |

## Known device-registration boundary

`copper_gauge`, `seared_fluid_cannon`, `seared_casting_tank`: source pages identify these; no tracked Horizon block/item declarations found. PR100 reads both ID forms. No actual icon outcome established.

Puny omitted sections need source material tier selection, armor/item registration, source modifier data and presentation, worktable and further-reading resources. Source can describe functionality not implemented by the host. See source section files in table; do not treat source recipes or descriptions as working mechanics.

## Static referenced-item candidates without direct registration

- `tconstruct:blank_red_sand_cast`
- `tconstruct:blank_sand_cast`
- `tconstruct:copper_gauge`
- `tconstruct:seared_casting_tank`
- `tconstruct:seared_fluid_cannon`

This scan checks declarations and explicit generator entries only; generated tags and overlays require verification in the staged runtime.

Overlay registration check (generated from the combined guide staging tree): `tconstruct:blank_sand_cast`, `tconstruct:blank_red_sand_cast`, and `tconstruct:puny_smelting` are in `items.json`; `tconstruct:copper_gauge`, `tconstruct:seared_fluid_cannon`, and `tconstruct:seared_casting_tank` are absent. Reader string/object parsing fix is in PR100. Overlay presence does not establish rendering or gameplay.

## Remaining Puny chapters: concrete work and acceptance

| Source section | Required data and reader behavior | Runtime acceptance |
| --- | --- | --- |
| `index.json` melee_harvest_materials / ranged_materials | Tier-2 predicates and source tags, material stat/trait/part visuals, translated names and source colors; do not reuse Materials and You's tier-1 filter. | Correct tier-2 members and visible stats/traits; page links and LAN match. |
| `index.json` armor_materials / armor | Original armor material-type resolver and conditional armor tag, original armor pages, parts and icons. | Only eligible material/armor entries show with correct labels, no fabricated items. |
| `sections/upgrades.json`, `sections/slotless.json` | Original section/page JSON, modifier detail and recipe/slot data, available modifier/item sprites and Mantle layouts. | Original titles, description and supported diagrams navigate without overflow; missing mechanics remain identified. |
| `sections/worktable.json` | Original worktable text, images and recipe/showcase formats plus separately registered available items. | Source explanation and illustrations navigate; no claim that described machines work without independent gameplay checks. |
| `sections/further_reading.json` | Original guide recipes, text and item icons; conditional missing-book outputs distinguished. | Correct text and registered outputs, unavailable book icons do not masquerade as functional. |

All paths are under `assets/tconstruct/book/puny_smelting/` at the source revision. See guide-pages.md for individual English pages, image IDs and local asset presence. This table scopes future work; it does not authorize or claim implementation.
