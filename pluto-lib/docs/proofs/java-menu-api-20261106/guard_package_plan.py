"""One read-only package plan and preflight group: continuous disk floor and bounded teardown."""
import json, os, signal, subprocess, sys, time
from pathlib import Path
out=Path(sys.argv[1]); command=sys.argv[2:]
floor=10*1024**3+128*1024**2
free=lambda: os.statvfs('.').f_bavail*os.statvfs('.').f_frsize
minimum=free(); start=time.monotonic(); stopped=minimum<floor+4*1024**2; timed_out=False

def terminate_group(p):
 for sig in (signal.SIGINT,signal.SIGTERM,signal.SIGKILL):
  try:os.killpg(p.pid,sig)
  except ProcessLookupError:break
  try:p.wait(timeout=1)
  except subprocess.TimeoutExpired:continue
  # Descendants must not outlive the leader after it responds to INT/TERM.
  try:os.killpg(p.pid,signal.SIGKILL)
  except ProcessLookupError:pass
  return

if stopped:
 out.with_suffix('.json').write_text(json.dumps({'exit_code':130,'guard_stopped':True,'minimum_free_bytes':minimum,'command':command},indent=2)+'\n'); raise SystemExit(130)
with out.with_suffix('.log').open('w') as log:
 p=subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 print(f'PID {p.pid}: {command}',flush=True)
 while p.poll() is None:
  minimum=min(minimum,free())
  if minimum<floor or time.monotonic()-start>120:
   timed_out=time.monotonic()-start>120;stopped=True;terminate_group(p);break
  time.sleep(.2)
 minimum=min(minimum,free())
 stopped=stopped or minimum<floor
 code=130 if stopped else p.returncode
 result={'exit_code':code,'guard_stopped':stopped,'minimum_free_bytes':minimum,'timed_out':timed_out,'elapsed_seconds':time.monotonic()-start,'command':command}
 out.with_suffix('.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result),flush=True)
 raise SystemExit(code)
