#!/usr/bin/env python3
"""Fail closed before any compilation except this one exact outer-tank test harness."""
import json
import os
from pathlib import Path
import sys

OUT = Path(__file__).resolve().parent
STAGE = OUT / 'cargo-tmp'
TARGET = Path('/workspace/scratch/c7b540ff3393/.work/official-advancement-recovery/host-target/debug/deps')

def values(args, flag):
    found = []
    for i, arg in enumerate(args):
        if arg == flag:
            found.append(args[i + 1] if i + 1 < len(args) else None)
        elif arg.startswith(flag + '='):
            found.append(arg[len(flag) + 1:])
    return found

def classify(args, cwd, manifest):
    if args == ['-vV'] or args == ['--version']:
        return 'version_query'
    # Cargo's capability query reads stdin and prints metadata, never emits code.
    prints = values(args, '--print')
    allowed_prints = {'file-names', 'sysroot', 'split-debuginfo', 'crate-name', 'cfg', 'target-libdir', 'target-list'}
    if prints and all(x in allowed_prints for x in prints):
        bare = []
        skip = False
        pairs = {'--crate-name', '--crate-type', '--print', '--target', '-W'}
        for arg in args:
            if skip:
                skip = False
                continue
            if arg in pairs:
                skip = True
            elif arg == '-' or not arg.startswith('-'):
                bare.append(arg)
            elif not (arg.startswith(('--print=', '--crate-type=', '--crate-name=', '--target=')) or arg == '-Wwarnings'):
                return 'denied_query_flag'
        if not skip and bare in ([], ['-']) and values(args, '--crate-name') in ([], ['___']):
            return 'print_query'
        return 'denied_query_shape'
    source = [a for a in args if a.endswith('.rs')]
    if values(args, '--crate-name') != ['outer_tank_patch_fixture'] or args.count('--test') != 1:
        return 'denied_other_compilation'
    if Path(cwd).resolve().parent != STAGE or not manifest or Path(manifest).resolve() != Path(cwd).resolve():
        return 'denied_package_path'
    if len(source) != 1 or (Path(cwd) / source[0]).resolve() != Path(cwd).resolve() / 'lib.rs':
        return 'denied_source_path'
    if values(args, '--emit') != ['dep-info,link'] or values(args, '--out-dir') != [str(TARGET)]:
        return 'denied_output'
    if any('incremental=' in arg for arg in args) or '--crate-type' in args or any(arg.startswith('--crate-type=') for arg in args):
        return 'denied_extra_codegen'
    return 'fixture_test_compilation'

def main():
    if len(sys.argv) < 2:
        return 90
    compiler, args = sys.argv[1], sys.argv[2:]
    decision = classify(args, os.getcwd(), os.environ.get('CARGO_MANIFEST_DIR'))
    if decision == 'fixture_test_compilation':
        try:
            fd = os.open(OUT / 'fixture-test-compile.claim', os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
            os.close(fd)
        except FileExistsError:
            decision = 'denied_second_compilation'
    with (OUT / 'rustc-invocations.jsonl').open('a') as output:
        output.write(json.dumps({'compiler': compiler, 'args': args, 'cwd': os.getcwd(), 'manifest': os.environ.get('CARGO_MANIFEST_DIR'), 'decision': decision}) + '\n')
    if decision not in {'version_query', 'print_query', 'fixture_test_compilation'}:
        print('Refused rustc invocation before execution: ' + decision, file=sys.stderr)
        return 91
    os.execv(compiler, [compiler, *args])

if __name__ == '__main__':
    raise SystemExit(main())
