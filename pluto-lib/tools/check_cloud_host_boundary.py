#!/usr/bin/env python3
"""Read-only provenance check for the exact official cloud-insertion boundary."""
import argparse
import hashlib
import json
from pathlib import Path
from prepare_render_fixture import clean_official


def check(horizon: Path):
    root = Path(__file__).resolve().parents[1]
    proof = json.loads((root / 'docs/cloud-insertion-proof.json').read_text())
    horizon = horizon.resolve()
    head = clean_official(horizon, proof['horizon_revision'])
    for relative, expected in proof['official_source_sha256'].items():
        source = horizon / relative
        if not source.resolve().is_relative_to(horizon):
            raise ValueError(f'Official source escapes checkout: {relative}')
        if hashlib.sha256(source.read_bytes()).hexdigest() != expected:
            raise ValueError(f'Official source changed: {relative}')
    return {'horizon_revision': head, 'verified_source_files': len(proof['official_source_sha256']),
            'scope': 'source provenance only; not a module/type/execution test'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(check(args.horizon_source), indent=2))
