"""Bounded artifact and subprocess identities; no native compilation or applying."""
import hashlib,io,json,os,stat,subprocess,zipfile
from pathlib import Path,PurePosixPath
MAIN='08afa992d5091e03360f3473ea93fd17aa793987'
HORIZON='624c4f6265c02296fbe8a80f029dd0d987fb94c0'
MIXINS='e8cd0aa38cffef49cb2e088ef716baa107148c02'
BUNDLE='4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d'
def sha(data):return hashlib.sha256(data).hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True,env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}).strip()
def git_state(root):
    gd=Path(git(root,'rev-parse','--absolute-git-dir'))
    selected={}
    for name in ['HEAD','index','packed-refs','FETCH_HEAD','config','shallow']:
        p=gd/name
        if p.exists():
            if not p.is_file() or p.is_symlink():raise ValueError('Unexpected Git state file')
            selected[name]=sha(p.read_bytes())
    if (gd/'refs').exists():
        for p in (gd/'refs').rglob('*'):
            if p.is_symlink():raise ValueError('Symlink in refs')
            if p.is_file():selected[p.relative_to(gd).as_posix()]=sha(p.read_bytes())
    return {'head':git(root,'rev-parse','HEAD'),'tree':git(root,'rev-parse','HEAD^{tree}'),
            'refs':git(root,'for-each-ref','--format=%(refname) %(objectname)'), 'files':selected}

def members(path_or_file):
    result={}
    with zipfile.ZipFile(path_or_file) as archive:
        if len(archive.infolist())>10000:raise ValueError('Too many ZIP members')
        if sum(x.file_size for x in archive.infolist())>64*1024**2:raise ValueError('ZIP exceeds input cap')
        for entry in archive.infolist():
            p=PurePosixPath(entry.filename)
            if (p.is_absolute() or not p.parts or any(q in ('','..','.') for q in entry.filename.split('/')) or
                '\\' in entry.filename or stat.S_IFMT(entry.external_attr>>16)!=stat.S_IFREG):
                raise ValueError('Unsafe or non-regular ZIP member')
            if entry.filename in result:raise ValueError('Duplicate ZIP member')
            result[entry.filename]=archive.read(entry)
    return result

def tracked(root,*paths):
    value=subprocess.check_output(['git','-C',str(root),'ls-files','-z','--',*paths],env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'})
    result=[]
    for v in value.split(b'\0'):
        if not v:continue
        p=v.decode();source=root/p
        if source.is_symlink() or not source.is_file():raise ValueError('Nonregular main tracked input')
        result.append((p,source))
    return result

def verify_main_bundle(root,archive):
    import tomllib
    manifest=tomllib.loads((root/'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text())
    compatibility=json.loads((root/'tconstruct/compatibility.json').read_text())
    if (compatibility['horizon_revision']!='0c7b5b61a5e746d621afffba919b12aeccf9c0fa' or
        compatibility['mixins_revision']!='ddd9700b21997c95d95ac6ee5ecafe3830cf622a' or
        manifest['compatibility']['horizon_revision']!=compatibility['horizon_revision'] or manifest['mod']['id']!='tconstruct'):
        raise ValueError('Main was repinned or substituted')
    value=members(archive)
    expected={'horizon-native.json':(json.dumps({'schema':1,'id':'tconstruct_native','version':manifest['mod']['version'],
        'game':'26.3','horizon':compatibility['horizon_revision'],'mixins':compatibility['mixins_revision']},indent=2)+'\n').encode()}
    for relative,p in tracked(root,'tconstruct/mixins/tconstruct'):
        if p.suffix not in ('.rs','.toml'):raise ValueError('Unexpected native file')
        inside=Path(relative).relative_to('tconstruct/mixins/tconstruct').as_posix()
        payload=p.read_bytes().replace(b'__horizon_mixins_tconstruct_',b'__horizon_mixins_tconstruct_native_')
        if inside=='horizon-mixin.toml':payload=payload.replace(b'id = "tconstruct"',b'id = "tconstruct_native"',1)
        expected['mixins/tconstruct_native/'+inside]=payload
    support={}
    for relative,p in tracked(root,'tconstruct/content','tconstruct/assets','tconstruct/third_party','tconstruct/docs','tconstruct/tools','pluto-lib/pluto_lib'):
        parts=Path(relative).parts
        if parts[:2]==('tconstruct','docs') and not p.name.endswith('.sha256'):continue
        if parts[:2]==('tconstruct','tools') and p.suffix!='.py':continue
        support[relative]=p.read_bytes()
    name='mixins/tconstruct_native/data/tconstruct-support.zip'
    if name not in value or members(io.BytesIO(value[name]))!=support:raise ValueError('Incomplete or changed inert support archive')
    expected[name]=value[name]
    if expected!=value:raise ValueError('Main package omitted, added or changed inputs')
    if len(value)!=499:raise ValueError('Unexpected main package size')
    return {'sha256':sha(archive.read_bytes()),'bytes':archive.stat().st_size,'members':len(value),
            'support_members':len(support),'header':json.loads(value['horizon-native.json'])}

def compose(main_archive,common_archive,destination):
    if sha(common_archive.read_bytes())!=BUNDLE:raise ValueError('Changed successful common bundle')
    groups=[members(main_archive),members(common_archive)]
    if len(groups[1])!=147:raise ValueError('Changed common member count')
    combined={};headers=[]
    for group in groups:
        headers.append(json.loads(group['horizon-native.json']))
        for name,payload in group.items():
            if name=='horizon-native.json':continue
            if not name.startswith('mixins/'):raise ValueError('Unexpected active bundle entry')
            relative=name[len('mixins/'):]
            if relative in combined:raise ValueError('Cross-bundle path collision')
            combined[relative]=payload
    destination.mkdir()
    for relative,payload in combined.items():
        p=destination/relative;p.parent.mkdir(parents=True,exist_ok=True)
        with p.open('xb') as stream:stream.write(payload)
    return {'headers':headers,'members':len(combined),'member_sha256':{p:sha(b) for p,b in sorted(combined.items())}}

def metadata_args(manifest):
    return ['metadata','--format-version','1','--no-deps','--manifest-path',str(manifest),'--offline','--locked']

def allowed(kind,args,manifest):
    return args==metadata_args(manifest) if kind=='cargo' else kind=='rustc' and args in [['--version'],['-vV']]

def write_wrappers(directory,manifest,real_cargo,real_rustc,log):
    directory.mkdir()
    for kind,real in [('cargo',real_cargo),('rustc',real_rustc)]:
        source='''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
kind=KIND
real=REAL
log=Path(LOG)
expected=EXPECTED
args=sys.argv[1:]
allow=(args==expected) if kind=='cargo' else args in [['--version'],['-vV']]
with log.open('a') as out:out.write(json.dumps({'kind':kind,'argv':args,'allowed':allow,'real':real})+'\\n')
if not allow:
 print('Plan-only wrapper rejected an unexpected tool invocation',file=sys.stderr)
 raise SystemExit(97)
os.execv(real,[real,*args])
'''.replace('KIND',repr(kind)).replace('REAL',repr(str(real))).replace('LOG',repr(str(log))).replace('EXPECTED',repr(metadata_args(manifest)))
        path=directory/kind;path.write_text(source);path.chmod(0o755)
    return {p.name:sha(p.read_bytes()) for p in directory.iterdir()}

def diagnostic(log):
    value=json.loads(log)
    if not isinstance(value,dict) or not isinstance(value.get('code'),str) or not value['code'].startswith('E_') or not isinstance(value.get('message'),str):
        raise ValueError('Not a complete official diagnostic object')
    if set(value)-{'code','message','file','mod_id','injection_id','target','selector'}:raise ValueError('Unexpected diagnostic fields')
    return value
