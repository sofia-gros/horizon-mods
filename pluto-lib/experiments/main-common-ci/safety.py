"""Exact read-only safety functions from successful outer runner e33fa328.

Original whole file SHA256 5203d61d9121eaa61d6dcd61d58591f3482926f9a954ec09d87a08724a7f7eab.
Copied function bodies unchanged; no cgroup or system settings are written.
"""
import os,re
from pathlib import Path

def memory_hierarchy(proc, root):
    """Accept only an unmasked cgroup2 mount exposing the real hierarchy root.

    CFTYPE_NOT_ON_ROOT checks cgroup_parent(), not the namespace root. Thus a
    delegated root retains cgroup.events/type. ENOENT is allowed only for the
    four root-only omissions, after mount/device/regular-node validation.
    Other topologies deliberately remain unsupported. This creates no limit.
    """
    import stat
    root = Path(root)
    if not root.is_absolute() or root.resolve() != root:
        raise ValueError('Symlink or unresolved cgroup mount')
    mounts = (proc/'self/mountinfo').read_text()
    rows = []
    for line in mounts.splitlines():
        before, separator, after = line.partition(' - ')
        fields, filesystem = before.split(), after.split()
        if not separator or len(fields) < 6 or len(filesystem) < 3:
            raise ValueError('Malformed mount identity')
        rows.append((fields, filesystem))
    owners = [(f, fs) for f, fs in rows if fs[0] == 'cgroup2']
    if len(owners) != 1:
        raise ValueError('Require one unambiguous cgroup2 mount')
    fields, _ = owners[0]
    if fields[3] != '/' or fields[4] != str(root):
        raise ValueError('Unsupported cgroup mount root or mount point')
    covering = [f for f, _ in rows if f[4] == str(root) or f[4].startswith(str(root)+'/')]
    if len(covering) != 1:
        raise ValueError('Masked or nested cgroup mount')
    if not re.fullmatch(r'[0-9]+:[0-9]+', fields[2]):
        raise ValueError('Malformed cgroup mount device')
    device = tuple(map(int, fields[2].split(':')))

    def checked(path, directory=False):
        info = path.lstat()
        kind = stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)
        if not kind or (os.major(info.st_dev), os.minor(info.st_dev)) != device:
            raise ValueError('Substituted cgroup node: '+str(path))
        return path

    checked(root, directory=True)
    controllers = checked(root/'cgroup.controllers').read_text().split()
    if 'memory' not in controllers:
        raise ValueError('Memory controller not observable at hierarchy root')
    for name in ['cgroup.events', 'cgroup.type', 'memory.max', 'memory.current']:
        try:
            (root/name).lstat()
        except FileNotFoundError:
            continue
        raise ValueError('Visible root is not the actual hierarchy root: '+name)

    membership = (proc/'self/cgroup').read_text()
    lines = membership.splitlines()
    if len(lines) != 1 or not lines[0].startswith('0::/'):
        raise ValueError('Require unique unified cgroup membership')
    relative = lines[0][3:]
    if (not re.fullmatch(r'/[A-Za-z0-9_.:@/-]*', relative) or '//' in relative or
        any(p in ('.', '..') for p in relative.split('/'))):
        raise ValueError('Unsupported or escaping cgroup membership')
    current = root / relative.lstrip('/')
    values = []
    while current != root:
        checked(current, directory=True)
        # Missing, permission-denied and malformed non-root values all reject.
        kind = checked(current/'cgroup.type').read_text().strip()
        if kind not in ('domain', 'domain threaded'):
            raise ValueError('Unsupported cgroup domain')
        checked(current/'cgroup.events').read_text()
        limit = checked(current/'memory.max').read_text().strip()
        usage = checked(current/'memory.current').read_text().strip()
        if (limit != 'max' and not re.fullmatch(r'[0-9]+', limit)) or not re.fullmatch(r'[0-9]+', usage):
            raise ValueError('Malformed cgroup memory observation')
        values.append((None if limit == 'max' else int(limit), int(usage)))
        current = current.parent
    if ((proc/'self/cgroup').read_text() != membership or
        (proc/'self/mountinfo').read_text() != mounts):
        raise ValueError('Cgroup topology changed during observation')
    return values

def check_memory(reserve, proc=Path('/proc'), root=Path('/sys/fs/cgroup')):
    """Observe both host availability and every enclosing cgroup-v2 limit.

    The true root lacks memory files by kernel design; delegated/hidden roots
    are not accepted. Thresholds are admission/stop limits, not hard caps or
    measured peak estimates. The native entry point is still unreachable.
    """
    mem = {}
    for line in (proc/'meminfo').read_text().splitlines():
        key, value = line.split(':', 1)
        if key in ('MemAvailable', 'MemTotal'): mem[key] = int(value.split()[0]) * 1024
    if set(mem) != {'MemAvailable', 'MemTotal'}: raise ValueError('Unknown RAM capacity')
    if not 0 <= mem['MemAvailable'] <= mem['MemTotal']:
        raise ValueError('Inconsistent host memory values')
    capacity, available = mem['MemTotal'], mem['MemAvailable']
    for limit, usage in memory_hierarchy(proc, root):
        if limit is not None:
            capacity = min(capacity, limit)
            available = min(available, max(0, limit-usage))
    if capacity < 6 * 1024**3 or available < reserve: raise RuntimeError('RAM/cgroup safety limit reached')
    return available

def memory_observation(proc=Path('/proc'), cgroup_root=Path('/sys/fs/cgroup')):
    """Read-only evidence, never an exception to the fail-closed memory guard."""
    def read(path):
        try:
            with path.open('r') as file: value=file.read(65537)
            if len(value)>65536:return {'error':'observation exceeds 64 KiB'}
            return {'text':value}
        except OSError as exc:return {'error':type(exc).__name__+': '+str(exc)}
    meminfo=read(proc/'meminfo')
    memory={}
    for line in meminfo.get('text','').splitlines():
        if line.startswith(('MemTotal:','MemAvailable:')):
            key,value=line.split(':',1);memory[key]=value.strip()
    self_group=read(proc/'self/cgroup');mounts=read(proc/'self/mountinfo')
    result={'admission':'not determined by this observation; guard remains independent',
            'host_memory':memory,'self_cgroup':self_group,'cgroup2_mounts':[],
            'visible_ancestors':[]}
    if 'error' in meminfo:result['host_memory_error']=meminfo['error']
    if 'error' in mounts:result['mount_error']=mounts['error']
    for line in mounts.get('text','').splitlines():
        before,separator,after=line.partition(' - ')
        if separator and after.split()[0]=='cgroup2':
            fields=before.split()
            result['cgroup2_mounts'].append({'root':fields[3],'mount_point':fields[4],
                                           'options':fields[5],'filesystem':after})
    entries=[line[3:] for line in self_group.get('text','').splitlines() if line.startswith('0::')]
    root=cgroup_root.resolve()
    if len(entries)!=1:result['hierarchy_error']='No unique observable cgroup-v2 membership';return result
    current=(root/entries[0].lstrip('/')).resolve()
    if not current.is_relative_to(root):result['hierarchy_error']='Membership path outside visible root';return result
    while True:
        facts={name:read(current/name) for name in ['memory.max','memory.current','cgroup.controllers','cgroup.events','cgroup.type']}
        result['visible_ancestors'].append({'path':str(current),'facts':facts})
        if current==root:break
        current=current.parent
    return result
