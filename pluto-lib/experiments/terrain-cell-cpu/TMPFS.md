# Blocked local alternative, not an execution result

The 2026-11-06 preparation was stopped **before any native launch** because this
environment does not expose its cgroup filesystem. Host MemAvailable does not
prove the container's effective limit. The runner now rejects admission when no
finite applicable cgroup limit is observable. A later visible bound still needs
an independently reviewed full-hierarchy/authority assessment before a slot is
granted; this runner does not establish that an outer hidden limit is absent.
Use the frozen manual CI path instead. No native plan, apply, compile or CPU
measurement was performed by this local alternative.

The manual CI checkpoint remains unchanged at `74d4cb7c`. This separate runner
uses exactly its frozen fixture, verifier and driver bytes and requires the
same ordinary v14 package. It runs only after explicit serial-slot release and
a check that the remote CPU job has not already started.

All generated exports, applied sources, Cargo target files, compiler temporary
files and logs belong to one new direct child of the separately mounted `/tmp`
tmpfs. The existing verified official Git checkout and Mixins binary are read
only. No existing target is copied, symlinked, removed or made writable. Cargo
uses the existing registry offline with the existing pinned toolchain. Cargo
can update its root access bookkeeping/locks; this is recorded separately, not
misrepresented as zero filesystem writes. The complete registry contents are
hashed before and after a successful run and must remain identical.

Admission and every expensive subprocess enforce three independent limits:

- Workspace filesystem: at least 10 GiB + 128 MiB free.
- `/tmp` filesystem: at least 512 MiB free.
- RAM: at least 1.5 GiB of the minimum of host MemAvailable and every visible
  applicable cgroup's remaining memory, including visible ancestor limits.
  If no finite cgroup bound is exposed, admission fails; host memory alone is
  not an approved substitute.

The process-group monitor checks every 250 ms, has a one-hour total deadline,
and interrupts only its own subprocess group on failure. If it does not exit
within 20 seconds, that same group is killed. It preserves all outputs and
records measured minima and the failure. These margins are safety stops, not
a guarantee that resource demand cannot spike between samples.

Compilation uses one job, no debug information, no incremental codegen and the
unoptimized test profile. It compiles the full actual native renderer unit-test
crate, then executes the one exact, non-skipping fixture in two separate
processes. It does not replace any native dependency with a model. Every
production composition guard and the test-module guard run before actual apply.
Applied source/Cargo.lock and original fixture inputs are sealed before and
after execution. No GPU context or production asset pack is exercised.

Example, after the environment's existing official development setup is loaded:

    python3 -B pluto-lib/experiments/terrain-cell-cpu/tmpfs.py \
      --official-git /path/to/verified-official-checkout \
      --cli /path/to/verified-e8cd-horizon-mixins \
      --overlay-root /path/to/workspace \
      --out /tmp/pluto-terrain-cpu-new-unique-name

The runner rejects a changed official origin/revision, CLI hash, fixture input
or ordinary package. Actual plan, compilation, CPU equality and timing results
remain pending until a terminal result is recorded. An interrupted run, a
zero-test pass or a source review does not satisfy those gates.
