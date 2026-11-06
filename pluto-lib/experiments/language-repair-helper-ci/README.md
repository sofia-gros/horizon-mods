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
