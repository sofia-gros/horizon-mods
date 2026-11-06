"""Pure checks derived from World probe checkpoint22c62f30.
The CI-only parser additionally admits one exact spanless abort summary.
No native execution, local paths, imports of experiment runners or runtime claims.
"""
import json
import shlex
SOURCE = "crates/horizon-client/src/game.rs"

def rustc_invocations(log):
    found = []
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:
            continue
        argv = shlex.split(line.split('Running `', 1)[1].rsplit('`', 1)[0])
        if '--crate-name' not in argv:
            continue
        found.append({'crate': argv[argv.index('--crate-name') + 1],
            'crate_type': argv[argv.index('--crate-type') + 1] if '--crate-type' in argv else None,
            'out_dirs': [x.split('=', 1)[1] for x in argv if x.startswith('OUT_DIR=')],
            'command': line})
    return found

def reject_diagnostic(messages, overlay_source):
    """Require exactly the prepared E0499, with primary span on the added call."""
    errors = []
    for line in messages.splitlines():
        obj = json.loads(line)  # Cargo JSON only; malformed/unrelated text fails closed.
        if obj.get('reason') == 'compiler-message' and obj['message']['level'] == 'error':
            errors.append(obj)
    summaries = [error for error in errors if error['message'].get('code') is None
        and error['message'].get('spans') == []
        and error['message'].get('message') == 'aborting due to 1 previous error']
    if len(summaries) > 1:
        raise ValueError('Duplicate compiler abort summary')
    errors = [error for error in errors if error not in summaries]
    if len(errors) != 1:
        raise ValueError('Expected exactly one compiler error')
    error = errors[0]
    msg = error['message']
    if (msg.get('code') or {}).get('code') != 'E0499':
        raise ValueError('Not the specified borrow rejection')
    if error['target']['name'] != 'horizon_client' or error['target']['kind'] != ['lib']:
        raise ValueError('Not the actual gameplay library')
    body = overlay_source.decode()
    marker = 'mod pluto_contact_negative_probe {'
    start = body.index(marker)
    call = body.index('        game.add_player_effect(effect);', start)
    line_number = body[:call].count('\n') + 1
    spans = [s for s in msg['spans'] if s['is_primary']]
    if len(spans) != 1:
        raise ValueError('Ambiguous primary diagnostic')
    span = spans[0]
    if not span['file_name'].replace('\\', '/').endswith(SOURCE):
        raise ValueError('Rejection outside the added Game source')
    if span['line_start'] != line_number or span['line_end'] != line_number:
        raise ValueError('Rejection at the wrong call')
    if not any('game.add_player_effect(effect);' in t['text'] for t in span['text']):
        raise ValueError('Missing exact rejected call')
    module_byte_start = len(body[:start].encode())
    if not (module_byte_start <= span['byte_start'] < span['byte_end'] <= len(overlay_source)):
        raise ValueError('Rejection outside the added module')
    return {'code': 'E0499', 'line': line_number, 'diagnostic': error,
        'optional_abort_summary': summaries[0] if summaries else None}
