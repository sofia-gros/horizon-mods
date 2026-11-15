"""Canonical guarded development build; not a distributable release or game test."""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def build(source, output, mixins, target):
    source, output, mixins, target = (p.resolve() for p in (source, output, mixins, target))
    if not output.is_relative_to(ROOT / '.local/clean-port') or output == ROOT / '.local/clean-port':
        raise ValueError('output must be a new isolated clean-port workspace')
    if output.exists():
        raise ValueError('output already exists; preserve prior evidence')
    if not target.is_relative_to(ROOT / '.local') or target == ROOT / '.local':
        raise ValueError('target must be a repository-local cache')
    if target.is_relative_to(output) or output.is_relative_to(target):
        raise ValueError('build cache and prepared sources must be separate')
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='0',
                       CARGO_PROFILE_DEV_DEBUG='0')
    commands = [
        [ROOT / 'pluto-lib/tools/plan_direct_host.py', '--source', source, '--output', output,
         '--mixins', mixins, '--check-contracts', '--check-api'],
        [ROOT / 'tools/check_clean_linkage.py', '--prepared', output],
        [ROOT / 'tools/check_game_entry.py', '--prepared', output],
        [ROOT / 'tools/check_game_entry.py', '--prepared', output, '--link', '--target-dir', target],
    ]
    for command in commands:
        result = subprocess.run([sys.executable, *map(str, command)], cwd=ROOT, env=environment)
        if result.returncode:
            return result.returncode
    print('Development executable linked; use run_clean_game.py with this prepared directory.')
    print('Official launcher execution, gameplay and release distribution are not verified by this command.')
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--mixins', required=True, type=Path)
    parser.add_argument('--target-dir', required=True, type=Path)
    args = parser.parse_args()
    try:
        return build(args.source, args.output, args.mixins, args.target_dir)
    except (OSError, ValueError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
