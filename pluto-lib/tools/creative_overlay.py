#!/usr/bin/env python3
"""Create a separate resource overlay using a demo-generated Creative inventory."""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('resources', type=Path)
parser.add_argument('creative', type=Path)
parser.add_argument('output', type=Path)
args = parser.parse_args()
source = args.resources.resolve(strict=True)
creative = args.creative.resolve(strict=True)
output = args.output.resolve()
if not source.is_dir() or output.exists() or output.is_relative_to(source) or source.is_relative_to(output):
    parser.error('output must be new and separate from the resource directory')
if not (source / 'creative-inventory.json').is_file():
    parser.error('resources must contain creative-inventory.json')
output.mkdir(parents=True)
for path in source.iterdir():
    target = creative if path.name == 'creative-inventory.json' else path.resolve()
    (output / path.name).symlink_to(target, target_is_directory=target.is_dir())
