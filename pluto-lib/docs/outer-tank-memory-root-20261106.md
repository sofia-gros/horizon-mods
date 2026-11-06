# Outer-tank memory guard: real-root distinction

Status at source checkpoint `527bd9cca2facb399072dcb7a15ed7a7f54656f3`: **source-only guard preparation, not native execution permission**. That checkpoint retained the diagnostic-only stop and no call to `execute_native_prepared`. A later source-only switch proposal is described in the CI README; it has not run. The frozen 160 Rust cases have not run; no cgroup, systemd unit, mount, limit, or local cache was changed.

## Why namespace roots are different

The official Linux documentation calls `cgroup.type`, `cgroup.events`, `memory.current`, and `memory.max` non-root interfaces. The implementation makes the relevant distinction precise:

- [Linux 6.11 file creation](https://github.com/torvalds/linux/blob/98f7e32f20d28ec452afb208f9cffc08448a2652/kernel/cgroup/cgroup.c#L4256-L4266) suppresses a `CFTYPE_NOT_ON_ROOT` file only when the actual cgroup has no parent. It does not compare the cgroup with a namespace root.
- [`cgroup.type` and `cgroup.events`](https://github.com/torvalds/linux/blob/98f7e32f20d28ec452afb208f9cffc08448a2652/kernel/cgroup/cgroup.c#L5217-L5260) carry that flag. So do [`memory.current` and `memory.max`](https://github.com/torvalds/linux/blob/98f7e32f20d28ec452afb208f9cffc08448a2652/mm/memcontrol.c#L4318-L4353).
- [A non-initial namespace mount](https://github.com/torvalds/linux/blob/98f7e32f20d28ec452afb208f9cffc08448a2652/kernel/cgroup/cgroup.c#L2181-L2221) returns the existing cgroup's dentry. It does not recreate the cgroup as a parentless object or remove those files. [Namespace path rendering](https://github.com/torvalds/linux/blob/98f7e32f20d28ec452afb208f9cffc08448a2652/kernel/cgroup/cgroup.c#L1895-L1912) explains why a reported mount-root string alone is insufficient.
- The same creation predicate and four flags were checked in Linux 6.8, commit [e8f897f4](https://github.com/torvalds/linux/tree/e8f897f4afef0031fe618a8e94127a0934896aba). This is source evidence for the kernel contract, not a measurement of the CI runner's exact kernel binary.

Therefore a normal delegated/namespaced non-root cgroup keeps the core `cgroup.type/events` files even if its virtual path is `/`. Missing memory-controller files alone cannot prove the true root: controller availability and unreadable/covered paths must also be considered.

## Bounded predicate prepared for review

The read-only `memory_hierarchy` helper supports only one narrow topology:

1. One cgroup2 mount, the fixed mount point, and reported root `/`. Reject duplicate, alternative-root, nested, or covering mount entries.
2. A regular directory with matching mountinfo major/minor device. No root/path/control-file symlinks or substituted device/type.
3. Readable root controllers including `memory`. All four root interfaces above must be absent with **ENOENT**. Permission failures are errors, not absence. Any one interface present rejects the true-root exception.
4. One unified, non-escaping membership path. Every non-root ancestor must have readable domain/type, events, max and current files on the same cgroup2 device. Missing, malformed or permission-denied data rejects.
5. Re-read membership and mountinfo before returning. Combine host available memory with every finite ancestor's remaining limit; the existing capacity/reserve thresholds remain unchanged.

The actual kernel root contributes no finite cgroup limit, but this is not an unlimited-memory claim: host availability and all non-root limits still apply. Unknown topology remains blocked. This does not defend against a compromised kernel, nor is a polling reserve a hard allocation ceiling.

The earlier [diagnostic receipt](outer-tank-memory-diagnostic-20261106.json) is consistent with this root contract. It did not record all mount/device checks now required, so it is **not** retroactively a passing admission result. The new guard has not run on that remote runner.

## Explicit limits: feasible mechanism, not applied

[systemd MemoryMax](https://github.com/systemd/systemd/blob/v256/man/systemd.resource-control.xml#L425-L450) configures a unit's cgroup memory hard limit; effective limits include parent slices. [MemorySwapMax](https://github.com/systemd/systemd/blob/v256/man/systemd.resource-control.xml#L455-L477) separately limits swap. Linux documents that `memory.max` may be exceeded transiently and can invoke the cgroup OOM killer; it is not a guarantee that a build completes.

Do not blindly wrap this runner with `systemd-run --scope`: its [default system slice placement](https://github.com/systemd/systemd/blob/v256/man/systemd-run.xml#L163-L190) need not keep the new unit below the current hosted-compute-agent service. Moving the task can discard an enclosing service limit. Per-process address-space limits likewise do not impose an aggregate compiler-child budget.

An explicit-cap variant would require separately reviewed authority to create the bounded scope, exact descendant placement and numeric limit/swap readback before any setup, preserved enclosing limits, and reliable group cleanup. No such write or admission has been attempted. The current approach only prepares the minimal read-only real-root distinction requested for review; it does not silently add a system setting change.

## Validation

The complete pure-Python runner suite passes **26 tests**, including nine added cases for real-root omissions, namespace-root core files, ancestor limits, permission failures, partial non-root facts, masked/device-mismatched mounts, symlinks, malformed paths/values, and membership changes. The files used in these cases are synthetic: they test parser and decision behavior, not kernel enforcement. The existing AST regression still proves zero native-draft call sites and unconditional diagnostic termination.

During source preparation, inserting the membership-change regression initially misplaced the prior test's final statements and produced one `NameError`; that test-only placement was corrected, then all 26 passed. No Rust or native operation was involved in either run. Independent final review and a distinct execution decision remain required.
