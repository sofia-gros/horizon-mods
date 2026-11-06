"""One bounded API proof group: continuous disk floor and bounded teardown."""
import json, os, signal, subprocess, sys, time
from pathlib import Path
floor=10*1024**3+128*1024**2
free=lambda: os.statvfs('.').f_bavail*os.statvfs('.').f_frsize

def signal_group(p,sig):
 try:os.killpg(p.pid,sig)
 except ProcessLookupError:pass

def terminate_group(p):
 for sig in (signal.SIGINT,signal.SIGTERM,signal.SIGKILL):
  signal_group(p,sig)
  try:p.wait(timeout=1);return
  except subprocess.TimeoutExpired:continue

def cleanup_group(p):
 # This Popen exclusively owns its new session/group. Always kill its descendants,
 # even after a normal leader exit or an exception. Never signal another group.
 signal_group(p,signal.SIGKILL)
 try:p.wait(timeout=1)
 except subprocess.TimeoutExpired:return False
 end=time.monotonic()+1
 while True:
  try:os.killpg(p.pid,0)
  except ProcessLookupError:return True
  if time.monotonic()>=end:return False
  time.sleep(.02)

def main(out,command):
 minimum=free();start=time.monotonic();stopped=minimum<floor+12*1024**2
 timed_out=False;group_clean=True;p=None;error=None;code=130
 if stopped:
  out.with_suffix('.json').write_text(json.dumps({'exit_code':130,'guard_stopped':True,'minimum_free_bytes':minimum,'command':command},indent=2)+'\n');return 130
 try:
  with out.with_suffix('.log').open('w') as log:
   p=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
   print(f'PID {p.pid}: {command}',flush=True)
   while p.poll() is None:
    minimum=min(minimum,free())
    if minimum<floor or time.monotonic()-start>150:
     timed_out=time.monotonic()-start>150;stopped=True;terminate_group(p);break
    time.sleep(.2)
   code=p.returncode
 except BaseException as exc:
  error=type(exc).__name__+': '+str(exc);stopped=True
 finally:
  if p is not None:group_clean=cleanup_group(p)
  minimum=min(minimum,free())
  stopped=stopped or minimum<floor or not group_clean
  code=130 if stopped else code
  result={'exit_code':code,'guard_stopped':stopped,'minimum_free_bytes':minimum,'timed_out':timed_out,'elapsed_seconds':time.monotonic()-start,'command':command,'owned_process_group_absent':group_clean,'error':error}
  out.with_suffix('.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)
 return code

if __name__=='__main__':raise SystemExit(main(Path(sys.argv[1]),sys.argv[2:]))
