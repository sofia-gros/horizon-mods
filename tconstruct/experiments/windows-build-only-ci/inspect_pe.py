"""Bounded direct import inspection, not DLL installation or transitive resolution."""
import hashlib,json,struct,sys
from pathlib import Path
def inspect(binary):
    def read(off,n):
     if off<0 or off+n>len(binary):raise ValueError('PE bounds')
     return binary[off:off+n]
    def u16(o):return struct.unpack('<H',read(o,2))[0]
    def u32(o):return struct.unpack('<I',read(o,4))[0]
    if read(0,2)!=b'MZ':raise ValueError('Missing DOS header')
    pe=u32(0x3c)
    if read(pe,4)!=b'PE\0\0':raise ValueError('Missing PE header')
    if u16(pe+4)!=0x8664:raise ValueError('Not x64')
    n=u16(pe+6);size=u16(pe+20);opt=pe+24
    if not(0<n<=96 and size>=112+14*8 and u16(opt)==0x20b):raise ValueError('PE32+ header bounds')
    image_base=struct.unpack('<Q',read(opt+24,8))[0]
    sections=[]
    for i in range(n):
     o=opt+size+i*40
     sections.append((u32(o+12),u32(o+16),u32(o+20)))
    def offset(rva,n=1):
     for address,size,ptr in sections:
      if address<=rva and rva-address+n<=size:
       off=ptr+rva-address;read(off,n);return off
     raise ValueError('Unmapped RVA')
    def string(rva):
     o=offset(rva);end=binary.find(b'\0',o,min(len(binary),o+261))
     if end<0:raise ValueError('Unbounded DLL name')
     return read(o,end-o).decode('ascii')
    def imports(index,entry_size,name_offset,delay=False):
     rva=u32(opt+112+index*8);length=u32(opt+116+index*8)
     if rva==0 and length==0:return []
     if not rva or length<entry_size or length>1024*1024:raise ValueError('Import table bounds')
     names=[]
     for i in range(min(129,length//entry_size)):
      o=offset(rva+i*entry_size,entry_size)
      if read(o,entry_size)==b'\0'*entry_size:return names
      address=u32(o+name_offset)
      if delay:
       flags=u32(o)
       if flags not in (0,1):raise ValueError('Delay flags')
       if not flags:address-=image_base
      names.append(string(address))
     raise ValueError('Import descriptors unterminated')
    return {'machine':'x86_64','format':'PE32+','imports':imports(1,20,12),'delay_imports':imports(13,32,4,True),'transitive_closure_verified':False}

if __name__=='__main__':
    import prepare
    p=Path(sys.argv[1]);before=prepare.record(p);binary=p.read_bytes()
    if len(binary)!=before['bytes'] or hashlib.sha256(binary).hexdigest()!=before['sha256']:raise ValueError('PE changed')
    result=inspect(binary)
    if prepare.record(p)!=before:raise ValueError('PE changed after inspection')
    print(json.dumps({**result,'file':before},sort_keys=True))
