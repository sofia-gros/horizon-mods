# Isolated terrain bounds comparison CI

Prepared source only. This workflow has not run. Production, old successful CPU
oracle, profile fixture, manifests and guards remain unchanged. Publication first
uses a non-trigger checkpoint; the dedicated push needs an independent review and
the coordinator's serial release.

Before this checkpoint, the 20 runner, 11 bounds-source and 8 host-composer pure
Python checks passed. Neither Rust profile nor any native test was executed.

The parent is `e6d82461c1035a05d557700704bf212c97f531a2`. Its excluded comparison
preserves the complete production `Batch` geometry module, extracts the exact old
predicate, and compares a smaller predicate valid only for that private type's
existing constructors. The generated std source is 24,080 bytes with SHA256
`c8efa6a8fc8b3293174ed7f33fd4ff8da38ae74405dec11475e8a10e2a9bd44b`.
Seven original geometry tests and seven comparison tests are included, with no
ignored/filtered cases. All 14 must appear in the exact terse inventory and pass.

The eight host-composer Python tests also reconstruct the original host bytes
after the single proposed call-site change. This is source composition only:
the host module is not compiled or installed in this job.

## Execution and proof boundaries

The public standard Ubuntu 24.04 runner uses a read-only token, exact dedicated
push branch and pinned checkout action. There is no Cargo, official-source fetch,
Mixins plan/apply, Game, Mesher, GPU, artifact upload, shared cache, sudo or apt.
Rustup installs only the official Rust 1.95.0 minimal toolchain. Setup and all
commands share the inherited continuous 10 GiB + 128 MiB disk floor on every
potential output filesystem, with an extra 2 GiB reserve before setup. Own process
groups are killed/reaped and checked absent after every phase, including success.

After 20 runner regressions, 11 unchanged bounds source checks and 8 unchanged
host-composer checks, direct rustc builds two fresh std test binaries. Their actual
argument vectors explicitly select opt-level 0 and 3, with debug assertions and
overflow checks enabled, debuginfo 0, codegen-units 16 and LTO off. This does not
parse arbitrary Cargo environment tokens as compiler arguments. The controller
records the exact compiler version, binary hash and compile command. A successful
command must produce a new regular executable; each binary is hashed before its
separate inventory and test processes and again afterward.

Only guarded compiler/linker/test processes inherit hard **per-process** limits:
1 GiB address space, 120 CPU seconds and 128 MiB per output file. A compile phase
also has a 240-second wall ceiling. These are not a cgroup aggregate memory limit,
and are not permission to run locally under the previously unknown container
limit. A resource failure is a failure, never a reason to relax the caps silently.
Rustup setup remains under disk and time guards, not those compiler limits.

Each of the four fixed micro cases reports seven paired totals for 4,096 calls,
128 warmup calls, observed clock resolution and an explicit inconclusive flag.
The parser recomputes that flag. Samples shorter than 1 ms or 100 observed clock
ticks are inconclusive. There is no speed assertion or performance win criterion.
Constant fixtures, compiler specialization and unit-call overhead limit what the
timings mean. Passing opt0/opt3 means only the enumerated private-Batch comparisons
passed, not arbitrary floating-point geometry or whole-mesher equivalence.

The complete original checkout includes untracked/ignored-file rejection and a
non-following file/link seal. The fixed source inventory, generated source,
compiler and both binaries are verified after success or failure. Every run emits
a false/partial final receipt on any setup, source, test, parser or verification
failure; no zero-test or Fresh-only success is possible. Raw phase output is held
only on the ephemeral runner and represented by hash/bytes plus selected results
and a bounded failure tail. It is not promised available after the run.

This is **not** the latest Horizon reload/profile gate. Historical runtime `43f`
and old CPU `1ac` evidence remain separate. Actual latest plan identifiers,
whole-host typing, renderer CPU differential/replay, real game assets and GPU
display still require their own future gates. No shipping optimization is made.
