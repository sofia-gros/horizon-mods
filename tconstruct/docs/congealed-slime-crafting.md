# Congealed Slime crafting

The four existing Congealed Slime blocks can be crafted from a 2×2 square of
matching slimeballs: Earth uses vanilla Slime Balls, and Sky, Ichor, and Ender
use their respective slimeballs. One block consumes four balls. The existing
unpacking recipe returns four balls.

The native recipes preserve the pinned Tinkers' Construct `f4e77d43` shapes,
outputs, and `forge:slimeball/<kind>` inputs. Source-exact native item tags are
included so ordinary crafting resolves the same default items. The tags remain
data-driven; a pack may add or replace their values. They are not the broader
`forge:slimeballs` tag, so the defaults do not allow mixing slime types.

This restores an ordinary crafting route to blocks already registered in the
port, including inputs used by existing slime-bottle brewing. It does not add
the separate sticky Sky/Ichor/Ender Slime blocks, change block physics, guarantee
natural acquisition of every slimeball, or implement additional advancement
triggers. Existing casting and unpacking are unchanged.

Run `python -m unittest discover -s tconstruct/tests -p 'test_congealed_slime_recipes.py'`.
The checks compare all four source recipes, native tags, inverse quantities,
and packaged recipe/tag/block/item endpoints. Actual crafting transactions,
live datapack reload, world persistence, and LAN are not covered by these
data checks.
