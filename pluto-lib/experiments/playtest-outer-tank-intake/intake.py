"""Exact three-hunk source intake only. No repository writes or native execution."""
import hashlib, json
from pathlib import Path
HERE=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def facts():return json.loads((HERE/'intake.json').read_text())
def transform(path, data, reverse=False):
    f=facts()['files'][path]
    src,dst=('after','before') if reverse else ('before','after')
    if not isinstance(data,bytes) or sha(data)!=f[src+'_sha256'] or len(data)!=f[src+'_bytes']:
        raise ValueError('Wrong exact source: '+path)
    out=data
    for op in (reversed(f['operations']) if reverse else f['operations']):
        old,new=op[src].encode(),op[dst].encode()
        if out.count(old)!=1 or new in out:raise ValueError('Ambiguous operation')
        out=out.replace(old,new,1)
    if sha(out)!=f[dst+'_sha256'] or len(out)!=f[dst+'_bytes']:
        raise ValueError('Postimage mismatch')
    return out
def verify_inputs(inputs, dependency):
    f=facts()
    if set(inputs)!=set(f['files']):raise ValueError('Wrong selected file set')
    if sha(dependency)!=f['dependency']['sha256']:raise ValueError('Dependency changed')
    output={p:transform(p,b) for p,b in inputs.items()}
    if {p:transform(p,b,True) for p,b in output.items()}!=inputs:raise ValueError('Inverse failed')
    return output
