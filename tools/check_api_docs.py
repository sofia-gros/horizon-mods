#!/usr/bin/env python3
r"""Every public PlutoLib item must have a section, with a code example, in pluto-lib/docs/api/*.md.

A section is a heading whose text is the item in backticks: `### \`Event::register\`` for a method or constant, `### \`ServerPlayer\``
for a type or function. The section must contain a rust code block (the blocks are compiled as doctests by
`cargo test --manifest-path pluto-lib/tests/pure/Cargo.toml --doc`). Every public field of a struct must be named in that struct's section.
Run it whenever PlutoLib's public API changes; it fails when documentation is missing, so API changes cannot skip the pages.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'pluto-lib/mod/api/src/pure'
PAGES = ROOT / 'pluto-lib/docs/api'

HEADING = re.compile(r'^#{2,4}\s+`([^`]+)`\s*$')
IMPL = re.compile(r'^impl(?:<[^>]*>)?\s+(?:(?P<trait>[\w:<>, ]+?)\s+for\s+)?(?P<ty>[A-Za-z_]\w*)')
ITEM = re.compile(r'^pub\s+(struct|enum|trait|type|static|fn|const)\s+([A-Za-z_]\w*)')
MEMBER = re.compile(r'^    pub\s+(?:const\s+)?(fn|const)\s+([A-Za-z_]\w*)')
FIELD = re.compile(r'^    pub\s+([a-z_]\w*)\s*:')


def public_items(path):
    """Returns (keys, fields): the documentation keys the file's public API needs, and {struct: [public fields]}."""
    keys, fields = [], {}
    text = path.read_text(encoding='utf-8')
    text = text.split('#[cfg(test)]', 1)[0]
    impl_type, struct = None, None
    for line in text.splitlines():
        if line.startswith('}'):
            impl_type, struct = None, None
            continue
        impl = IMPL.match(line)
        if impl:
            impl_type = None if impl.group('trait') else impl.group('ty')
            struct = None
            continue
        if impl_type:
            member = MEMBER.match(line)
            if member:
                keys.append(f'{impl_type}::{member.group(2)}')
            continue
        item = ITEM.match(line)
        if item and not line.startswith('pub('):
            kind = item.group(1)
            keys.append(item.group(2))
            struct = item.group(2) if kind == 'struct' else None
            continue
        if struct:
            field = FIELD.match(line)
            if field:
                fields.setdefault(struct, []).append(field.group(1))
    return keys, fields


def sections(pages):
    """{key: section text} over all pages."""
    found = {}
    for page in pages:
        key, body = None, []
        for line in page.read_text(encoding='utf-8').splitlines():
            heading = HEADING.match(line)
            if heading or line.startswith('#'):
                if key:
                    found[key] = '\n'.join(body)
                key, body = (heading.group(1) if heading else None), []
            elif key:
                body.append(line)
        if key:
            found[key] = '\n'.join(body)
    return found


def problems():
    pages = sorted(PAGES.glob('*.md'))
    found = sections(pages)
    issues, expected = [], set()
    for path in sorted(SOURCE.rglob('*.rs')):
        keys, fields = public_items(path)
        rel = path.relative_to(ROOT).as_posix()
        for key in keys:
            expected.add(key)
            section = found.get(key)
            if section is None:
                issues.append(f'{rel}: `{key}` has no section in pluto-lib/docs/api')
            elif '```rust' not in section:
                issues.append(f'{rel}: the section of `{key}` has no ```rust example')
        for struct, names in fields.items():
            section = found.get(struct, '')
            for name in names:
                if not re.search(rf'\b{re.escape(name)}\b', section):
                    issues.append(f'{rel}: field `{struct}.{name}` is not named in the section of `{struct}`')
    for key in sorted(set(found) - expected):
        issues.append(f'pluto-lib/docs/api: section `{key}` documents nothing public (renamed or removed?)')
    return issues


def main():
    issues = problems()
    for issue in issues:
        print(issue, file=sys.stderr)
    print(f'API documentation: {len(issues)} problem(s)')
    return 1 if issues else 0


if __name__ == '__main__':
    sys.exit(main())
