"""One explicitly authorized temporary hook deferral; no save or helper changes."""
import hashlib, json, tomllib
from pathlib import Path
HERE=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def inputs():return json.loads((HERE/'intake.json').read_text())
def transform(body):
    block=inputs()['removed_block'].encode()
    if body.count(block)!=1:raise ValueError('Absent or ambiguous exact hook block')
    parsed=tomllib.loads(body.decode());wanted=inputs()['hook']
    rows=parsed.get('injection',[])
    if sum(x['id']==wanted for x in rows)!=1:raise ValueError('Ambiguous injection identity')
    position=body.index(block);after=body[:position]+body[position+len(block):]
    expected=dict(parsed);expected['injection']=[x for x in rows if x['id']!=wanted]
    if tomllib.loads(after.decode())!=expected:raise ValueError('Non-target declaration changed')
    if after[:position]+block+after[position:]!=body:raise ValueError('Full inverse failed')
    return after
def apply_exact(body,helper):
    config=inputs()
    if sha(body)!=config['before']['sha256'] or len(body)!=config['before']['bytes']:raise ValueError('Wrong integration input')
    if sha(helper)!=config['preserved_helper']['sha256']:raise ValueError('Ordinary helper changed')
    after=transform(body)
    if sha(after)!=config['after']['sha256'] or len(after)!=config['after']['bytes']:raise ValueError('Wrong output')
    return after
def restore_exact(body):
    config=inputs()
    if sha(body)!=config['after']['sha256']:raise ValueError('Wrong deferred output')
    offset=config['block_offset'];block=config['removed_block'].encode()
    before=body[:offset]+block+body[offset:]
    if sha(before)!=config['before']['sha256']:raise ValueError('Wrong restoration')
    return before
