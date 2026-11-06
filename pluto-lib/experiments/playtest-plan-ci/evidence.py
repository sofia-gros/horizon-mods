"""Four allowlisted public diagnostic outputs; no environment or arbitrary file upload."""
import hashlib,json,os,re,stat
from pathlib import Path

MIB=1024**2
LIMITS={'plan.json':16*MIB,'receipt.json':2*MIB,'package-maps.json':2*MIB,'manifest.json':MIB}
MAX_TOTAL=21*MIB
sha=lambda b:hashlib.sha256(b).hexdigest()

def encode(value):return (json.dumps(value,sort_keys=True,indent=2,allow_nan=False)+'\n').encode()
def no_links(path):
    if any(p.is_symlink() for p in (path,*path.parents)):raise ValueError('Evidence symlink forbidden')
def read_regular(path,limit):
    no_links(path)
    fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
    try:
        st=os.fstat(fd)
        if not stat.S_ISREG(st.st_mode) or not 0<st.st_size<=limit or st.st_nlink!=1:raise ValueError('Evidence input must be bounded unlinked regular file')
        chunks=[];total=0
        while True:
            part=os.read(fd,min(MIB,limit+1-total))
            if not part:break
            chunks.append(part);total+=len(part)
            if total>limit:raise ValueError('Evidence input grew beyond limit')
        after=os.fstat(fd)
        if (st.st_dev,st.st_ino,st.st_size,st.st_mtime_ns)!=(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns) or total!=st.st_size:raise ValueError('Evidence input changed')
        return b''.join(chunks)
    finally:os.close(fd)

def paths(base,head):
    if not re.fullmatch('[0-9a-f]{40}',head or ''):raise ValueError('Invalid evidence head')
    base=Path(base);no_links(base)
    if not base.is_absolute() or not base.is_dir():raise ValueError('Invalid runner temporary directory')
    return base/('playtest-complete-plan-'+head)/'plan.json',base/('playtest-plan-evidence-'+head)

def collect(report,plan,guard):
    gates=report.get('gates',{})
    maps={key:gates[key] for key in ('fixed_checkouts','tconstruct_bundle','pluto_bundle','combined_payload') if key in gates}
    data={'package-maps.json':encode(maps)}
    status={'plan_present':False,'plan_matches_accepted_receipt':False,'files':{},'scope':'Plan and public receipt/maps only; not compatibility or build proof.'}
    no_links(plan)
    if plan.exists():
        guard(LIMITS['plan.json'])
        b=read_regular(plan,LIMITS['plan.json'])
        expected=gates.get('plan_diagnostic',{})
        if expected.get('accepted'):
            if (len(b),sha(b))!=(expected.get('plan_bytes'),expected.get('plan_sha256')):raise ValueError('Preserved plan differs from accepted receipt')
            status['plan_matches_accepted_receipt']=True
        data['plan.json']=b;status['plan_present']=True
    elif report.get('plan_accepted') is True:raise ValueError('Accepted plan is missing')
    for name,b in data.items():
        if not 0<len(b)<=LIMITS[name]:raise ValueError('Evidence payload exceeds file limit')
        status['files'][name]={'bytes':len(b),'sha256':sha(b)}
    return data,status

def persist(report,base,guard):
    """Called after diagnostic teardown/final seals, including failed diagnostics."""
    plan,destination=paths(base,report.get('head'))
    no_links(destination)
    if destination.exists():raise ValueError('Evidence destination already exists')
    data,status=collect(report,plan,guard)
    report['artifact_preservation']=status
    data['receipt.json']=encode(report)
    manifest={'schema':1,'head':report['head'],'plan_present':status['plan_present'],
      'source_verified':report.get('source_verified',False),'probe_completed':report.get('probe_completed',False),
      'files':{name:{'bytes':len(b),'sha256':sha(b)} for name,b in sorted(data.items())}}
    data['manifest.json']=encode(manifest)
    if set(data)-set(LIMITS) or sum(map(len,data.values()))>MAX_TOTAL:raise ValueError('Evidence total budget exceeded')
    for name,b in data.items():
        if not 0<len(b)<=LIMITS[name]:raise ValueError('Evidence output file budget exceeded')
    guard(sum(map(len,data.values())))
    destination.mkdir(mode=0o700)
    for name,b in data.items():
        guard(len(b));p=destination/name
        with p.open('xb') as f:f.write(b);f.flush();os.fsync(f.fileno())
        if read_regular(p,LIMITS[name])!=b:raise ValueError('Evidence readback mismatch')
    fd=os.open(destination,os.O_RDONLY|os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)
    if {p.name for p in destination.iterdir()}!=set(data):raise ValueError('Unexpected evidence file')
    guard(0)
    return destination,manifest

def verify_saved(destination):
    """Use after archive extraction; does not extract or follow symlinks."""
    destination=Path(destination);no_links(destination)
    m=json.loads(read_regular(destination/'manifest.json',LIMITS['manifest.json']))
    if (not isinstance(m,dict) or set(m)!={'schema','head','plan_present','source_verified','probe_completed','files'}
            or type(m['schema']) is not int or m['schema']!=1 or not isinstance(m['head'],str)
            or not re.fullmatch('[0-9a-f]{40}',m['head']) or any(type(m[k]) is not bool for k in ('plan_present','source_verified','probe_completed'))
            or not isinstance(m['files'],dict)):
        raise ValueError('Invalid saved evidence manifest schema/head/types')
    names=set(m['files']);expected=names|{'manifest.json'}
    if (not {'receipt.json','package-maps.json'}.issubset(names) or 'manifest.json' in names
            or m['plan_present']!=('plan.json' in names) or expected-set(LIMITS)
            or {p.name for p in destination.iterdir()}!=expected):raise ValueError('Wrong saved evidence file set')
    total=0
    for name,v in m['files'].items():
        if (not isinstance(v,dict) or set(v)!={'bytes','sha256'} or type(v['bytes']) is not int or not 0<v['bytes']<=LIMITS[name]
                or not isinstance(v['sha256'],str) or not re.fullmatch('[0-9a-f]{64}',v['sha256'])):raise ValueError('Invalid saved file manifest')
        b=read_regular(destination/name,LIMITS[name]);total+=len(b)
        if (len(b),sha(b))!=(v['bytes'],v['sha256']):raise ValueError('Saved evidence content mismatch')
    if total+len(read_regular(destination/'manifest.json',LIMITS['manifest.json']))>MAX_TOTAL:raise ValueError('Saved evidence total exceeded')
    receipt=json.loads(read_regular(destination/'receipt.json',LIMITS['receipt.json']))
    if (receipt.get('head')!=m['head'] or receipt.get('source_verified',False)!=m['source_verified']
            or receipt.get('probe_completed',False)!=m['probe_completed']):raise ValueError('Saved receipt identity/status mismatch')
    if receipt.get('plan_accepted') is True:
        expected_plan=receipt.get('gates',{}).get('plan_diagnostic',{})
        entry=m['files'].get('plan.json',{})
        if expected_plan.get('accepted') is not True or (entry.get('bytes'),entry.get('sha256'))!=(expected_plan.get('plan_bytes'),expected_plan.get('plan_sha256')):
            raise ValueError('Saved accepted plan differs from receipt')
    return m
