# Language repair helper14 CI source

Dedicated public push-only, contents:read, standard Ubuntu job. This checkpoint
is nontrigger; execution needs a separate coordinator release. No native job has
run for this source. The frozen helper source is from 9756 and placement scaffold
from 489f, with the successful Pack8 controller utilities imported byte-identical.

This job has **no Mixins planner or apply** and does not compile native Horizon.
It tests only the exact same merge.rs in a standalone crate using actual serde
and serde_json identities locked by official Horizon f4dc. It checks the native
manifest's dependency requirements but does not pretend a native adapter used
the helper. Writer/stamp placement and actual build/cache remain separate gates.

The official checkout is exact and clean, including untracked/ignored files.
Its lock seeds the fixture before offline resolution. Every resolved package
identity must be in the official lock, except the one fixture package; every
local metadata package must be that fixture. No alternate JSON parser, mock
native types, registry override or new runtime dependency is introduced.

All fourteen test names are required once with zero ignored/filtered tests.
The actual cold serde/serde_json and --test fixture compiler paths must match
metadata. Compilation finishes before the generated inputs are sealed; the
subsequent locked/offline test command rejects any rustc recompile in its full
log. That is detection after invocation, not prevention of a rustc launch.
Source, complete fixture/metadata/lock file set and OUT_DIR are rechecked afterward.
As with the inherited utility, seals do not claim permission-mode/empty-dir
identity. Complete raw logs are ephemeral; console digests are not retrievable
raw artifacts. Failure receipts remain false with available completed gates.

The inherited runner maintains the normal 10 GiB + 128 MiB floor plus setup
reserve and kills its process group. The preparer's Git/metadata children stay
in that group and cannot launch in a standalone nonleader context. No local
native execution, cache reuse/upload, permissions change, billing change,
production activation, Game/UI or pack-priority repair result is authorized by
this source-only checkpoint.


## First attempt and parser-only correction

Run 42214234395 at `3c105dbb` failed after the locked cold `--no-run`
compile phase exited 0. The witness parser rejected serde before any of the
fourteen tests ran. Final receipt is passed=false, verified=true; generated
OUT_DIR was not yet sealed. Raw compiler-phase bytes were not retained in the
bounded console, so the exact offending argv is unknown.

The former parser incorrectly treated every verbose token ending in `.rs` as
a source, including legal Cargo environment prefixes. Serde v1.0.229 declares
`homepage = "https://serde.rs"` in its public manifest. This is a concrete
parser defect and a plausible cause, not a recovered offending invocation.
The correction strips only leading shell variable assignments, requires one
actual rustc executable token, and checks source arguments after that token.
No dependency/source-path match, test flag, complete witness set or final seal
is relaxed. A failure now emits only bounded crate/path/shape plus argv hash,
not all environment values. Three new pure regressions cover `.rs` values,
`RUSTC=/.../rustc`, duplicate executables and bounded mismatch diagnostics.
Helper, fixture, lock policy, workflow and fourteen Rust tests are unchanged.
