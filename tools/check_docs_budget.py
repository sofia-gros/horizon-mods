#!/usr/bin/env python3
"""Keep the documentation small: every tracked markdown file must be on the allow list.

Documents here are few and current; per-feature detail lives in code (rustdoc, tests), not in prose. Adding a document is a
deliberate act: put it on ALLOWED in this file in the same change, and say why in the pull request.
"""
import subprocess
import sys
from fnmatch import fnmatch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWED = [
    'README.md', 'README.ja.md', 'AGENTS.md', 'CLAUDE.md',
    'docs/architecture.md', 'docs/development.md', 'docs/releasing.md', 'docs/roadmap.md', 'docs/codex.md', 'docs/migration/pr-classification.md',
    'pluto-lib/README.md', 'pluto-lib/docs/design-principles.md', 'pluto-lib/docs/fabric-api-design.md',
    'pluto-lib/docs/fabric-api-hook-map.md', 'pluto-lib/docs/support-matrix.md', 'pluto-lib/docs/api/*.md',
    'tconstruct/README.md',
    # licence and attribution notices are legal text, not documentation
    '*NOTICE.md', '*NOTICES.md', '*LICENSE.md',
]
MAX_ALLOWED_BYTES = 80_000  # a single document this large is no longer readable by a person


def violations(paths, sizes=None):
    found = []
    for path in paths:
        if not any(fnmatch(path, pattern) for pattern in ALLOWED):
            found.append(f'{path}: not on the documentation allow list (tools/check_docs_budget.py); update an existing document instead')
        elif sizes and sizes.get(path, 0) > MAX_ALLOWED_BYTES and not path.endswith('hook-map.md') and 'pr-classification' not in path:
            found.append(f'{path}: {sizes[path]} bytes is too long for a document a person reads')
    return found


def main():
    out = subprocess.run(['git', 'ls-files', '-z', '--', '*.md'], cwd=ROOT, capture_output=True, text=True, encoding='utf-8', check=True).stdout
    paths = [p for p in out.split('\0') if p]
    sizes = {p: (ROOT / p).stat().st_size for p in paths if (ROOT / p).exists()}
    found = violations(paths, sizes)
    for line in found:
        print(line, file=sys.stderr)
    print(f'documentation budget: {len(paths)} markdown file(s), {len(found)} problem(s)')
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main())
