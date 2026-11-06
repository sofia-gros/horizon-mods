"""Own two-process Job Object only. No admin changes, discovery or foreign PIDs."""
import ctypes as C
import os
import time

U32=C.c_uint32; U64=C.c_uint64; HANDLE=C.c_void_p
class Basic(C.Structure):
    _fields_=[('process_time',C.c_int64),('job_time',C.c_int64),('flags',U32),
              ('min_working',C.c_size_t),('max_working',C.c_size_t),('active',U32),
              ('affinity',C.c_size_t),('priority',U32),('scheduling',U32)]
class IO(C.Structure):
    _fields_=[('value'+str(i),U64) for i in range(6)]
class Limits(C.Structure):
    _fields_=[('basic',Basic),('io',IO),('process_memory',C.c_size_t),('job_memory',C.c_size_t),
              ('peak_process',C.c_size_t),('peak_job',C.c_size_t)]

class Job:
    def __init__(self):
        if os.name!='nt' or C.sizeof(C.c_void_p)!=8:raise RuntimeError('Windows x64 Job Object required')
        self.api=C.WinDLL('kernel32',use_last_error=True); self.handle=None
        prototypes={
            'CreateJobObjectW':([C.c_void_p,C.c_wchar_p],HANDLE),
            'SetInformationJobObject':([HANDLE,C.c_int,C.c_void_p,U32],C.c_int),
            'AssignProcessToJobObject':([HANDLE,HANDLE],C.c_int),
            'TerminateJobObject':([HANDLE,U32],C.c_int),
            'QueryInformationJobObject':([HANDLE,C.c_int,C.c_void_p,U32,C.POINTER(U32)],C.c_int),
            'CloseHandle':([HANDLE],C.c_int)}
        for name,(args,result) in prototypes.items():
            fn=getattr(self.api,name);fn.argtypes=args;fn.restype=result
        self.handle=self.api.CreateJobObjectW(None,None)
        if not self.handle:raise C.WinError(C.get_last_error())
        info=Limits();info.basic.flags=0x2000|0x0008 # KILL_ON_JOB_CLOSE | ACTIVE_PROCESS
        info.basic.active=2 # barrier worker + one exact parser/test process
        if not self.api.SetInformationJobObject(self.handle,9,C.byref(info),C.sizeof(info)):
            error=C.WinError(C.get_last_error());self.close();raise error
    def assign(self,process):
        # Borrow the handle returned by this exact Popen/CreateProcess. Never reopen
        # a PID, so early worker failure cannot turn assignment into a PID-reuse race.
        handle=HANDLE(int(process._handle))
        if not self.api.AssignProcessToJobObject(self.handle,handle):raise C.WinError(C.get_last_error())
    def terminate(self):
        if not self.api.TerminateJobObject(self.handle,1):raise C.WinError(C.get_last_error())
    def empty(self):
        # Header + at most two IDs; do not retain or print any process ID.
        data=C.create_string_buffer(8+2*C.sizeof(C.c_size_t));returned=U32()
        if not self.api.QueryInformationJobObject(self.handle,3,data,len(data),C.byref(returned)):
            raise C.WinError(C.get_last_error())
        return int.from_bytes(data.raw[:4],'little')==0
    def wait_empty(self,guard):
        end=time.monotonic()+2
        while time.monotonic()<end:
            guard()
            if self.empty():return
            time.sleep(0.02)
        raise RuntimeError('Own Job Object did not become empty')
    def close(self):
        if self.handle:
            handle=self.handle;self.handle=None
            if not self.api.CloseHandle(handle):raise C.WinError(C.get_last_error())
