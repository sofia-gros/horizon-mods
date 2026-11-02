import struct,zlib,gzip,json,sys
p=sys.argv[1];b=open(p,'rb').read();off=int.from_bytes(b[:3],'big')*4096;n=int.from_bytes(b[off:off+4],'big');typ=b[off+4];raw=b[off+5:off+4+n];d=(zlib.decompress(raw) if typ==2 else gzip.decompress(raw) if typ==1 else raw);pos=0
def read(n):
 global pos
 v=d[pos:pos+n];pos+=n;return v
def num(fmt):return struct.unpack('>'+fmt,read(struct.calcsize(fmt)))[0]
def string():return read(num('H')).decode('utf-8','replace')
def payload(t):
 if t==1:return num('b')
 if t==2:return num('h')
 if t==3:return num('i')
 if t==4:return num('q')
 if t==5:return num('f')
 if t==6:return num('d')
 if t==7:return f'<bytes {len(read(num("i")))}>'
 if t==8:return string()
 if t==9:
  k=num('B');n=num('i');return [payload(k) for _ in range(n)]
 if t==10:
  v={}
  while (k:=num('B')):
   key=string();v[key]=payload(k)
  return v
 if t in (11,12):
  n=num('i');read(n*(4 if t==11 else 8));return f'<array {n}>'
 raise ValueError(t)
t=num('B');root=string();data=payload(t)
for e in data.get('block_entities',[]):
 if (e.get('x'),e.get('y'),e.get('z')) in ((0,72,2),(0,71,2)): print(json.dumps(e,ensure_ascii=False))
