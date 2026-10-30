#!/usr/bin/env python3
"""Compare the Rust prototype with Java HashMap for the three pinned sources."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
JARS = [('sha1', '0c3ec587af28e5a785c0b4a7b8a30f9a8f78f838'),
        ('sha256', '2b0e97b22fd4d043dbbcc8e0a44ef0d58915689ed9224e4cd28d5c4976a16bcc')]

def recipe_id(path):
    parts = path.split('/')
    if len(parts) >= 4 and parts[0] == 'data' and parts[2] == 'recipes' and path.endswith('.json'):
        return parts[1] + ':' + '/'.join(parts[3:])[:-5]
    return None

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('minecraft_jar', type=Path)
    p.add_argument('forge_jar', type=Path)
    p.add_argument('tconstruct_checkout', type=Path)
    args = p.parse_args()
    revision = subprocess.run(['git', '-C', str(args.tconstruct_checkout), 'cat-file', '-e', REVISION + '^{commit}'])
    if revision.returncode: p.error('TConstruct checkout lacks the pinned commit')
    recipes = {}
    for jar, (algorithm, expected) in zip((args.minecraft_jar, args.forge_jar), JARS):
        with jar.open('rb') as source: digest = hashlib.file_digest(source, algorithm).hexdigest()
        if digest != expected: p.error(f'{jar} does not match its pinned hash')
        with zipfile.ZipFile(jar) as archive:
            for info in archive.infolist():
                identifier = recipe_id(info.filename)
                if identifier is None or info.is_dir(): continue
                try: json.loads(archive.read(info))
                except (ValueError, UnicodeDecodeError): continue
                recipes[identifier] = None
    prefix = 'src/generated/resources/'
    process = subprocess.Popen(['git', '-C', str(args.tconstruct_checkout), 'archive', REVISION, prefix + 'data'], stdout=subprocess.PIPE)
    with tarfile.open(fileobj=process.stdout, mode='r|') as archive:
        for info in archive:
            identifier = recipe_id(info.name.removeprefix(prefix))
            if not info.isfile() or identifier is None: continue
            try: json.load(archive.extractfile(info))
            except (ValueError, UnicodeDecodeError): continue
            recipes[identifier] = None
    if process.wait() != 0: raise RuntimeError('git archive failed')
    ids = list(recipes)
    with tempfile.TemporaryDirectory(prefix='recipe-order-') as temp:
        binary = Path(temp) / 'recipe-order'
        subprocess.run(['rustc', str(ROOT / 'tests/rust/recipe_order_dump.rs'), '-o', str(binary)], check=True)
        subprocess.run(['javac', '-d', temp, str(ROOT / 'tests/reference/RecipeOrderOracle.java')], check=True)
        rust = subprocess.check_output([str(binary)], input='\n'.join(ids) + '\n', text=True)
        java = subprocess.check_output(['java', '-cp', temp, 'RecipeOrderOracle', *ids], text=True)
    if rust != java: raise RuntimeError('Rust order differs from the Java oracle')
    print(f'PASS: {len(ids)} recipe IDs; order SHA-256 {hashlib.sha256(rust.encode()).hexdigest()}')

if __name__ == '__main__': main()
