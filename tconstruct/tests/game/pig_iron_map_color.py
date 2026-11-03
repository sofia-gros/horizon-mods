"""Check all prepared Pig Iron states use the source pink map color.

Run against a freshly prepared resource root produced from this checkout, not
an old cache: python pig_iron_map_color.py /path/to/prepared/resources
"""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
facts = json.loads((root / 'facts.json').read_text())
gameplay = json.loads((root / 'gameplay-facts.json').read_text())['states']
render = json.loads((root / 'render-definitions.json').read_text())
pig = render['tconstruct:pig_iron_block']['states']
pink = gameplay[facts['defaults']['minecraft:pink_wool']]['map_color']
white = gameplay[facts['defaults']['minecraft:white_glazed_terracotta']]['map_color']
assert pink == 15892389 and white != pink, (pink, white)
assert len(pig) == 4, pig
assert all(gameplay[state]['map_color'] == pink for state in pig)
print('PASS: all four Pig Iron states use the source pink map color')
