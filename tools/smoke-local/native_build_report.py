"""Reject native artifacts that do not prove the requested build inputs."""
import json
from pathlib import Path


def require_report(output, horizon, mixins, required_mods):
    report = json.loads((Path(output) / 'native-mods.json').read_text(encoding='utf-8'))
    actual = (report.get('horizon'), report.get('horizon_mixins'))
    if actual != (horizon, mixins):
        raise ValueError(f'native build pins differ: expected {(horizon, mixins)}, got {actual}')
    built = {mod['id'] for mod in report.get('mods', [])}
    if report.get('left_out') != [] or not set(required_mods) <= built:
        raise ValueError('native build omitted required mods or has no complete inclusion report')
    return report
