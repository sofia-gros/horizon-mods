# Guide identity helper CI

This dedicated public push branch compiles the exact production BookId and page
index helpers once with Rust 1.95.0. It checks the complete 21-test inventory, then
executes BookId5 and modifier-anchor5 as two separate processes against the same
sealed executable. Each group must have exactly five named successes and sixteen
filtered tests. The eleven unrelated existing index tests are not executed.

The workflow has read-only repository permission, an exact public-repository and
branch gate, no caches/artifacts/manual dispatch, and no custom runner. All setup
and child processes use the continuous 10 GiB + 128 MiB floor on the workspace,
output, root and Rustup filesystems. Setup first requires another 2 GiB of free
space. Output ownership is exclusive; timeout/disk failures stop the own process
group, including residual children after its leader exits. Source file-set/byte
seals and executable seals are checked after compilation and execution. A failed
or incomplete gate cannot produce a successful final receipt.

Rustup installs only the pinned minimal official toolchain. There is no Cargo,
apt, Horizon/Mixins download, native resource loading, Mod application or native
host compilation. The fixture input and three source files remain exactly those
published at d9f8e6ba50fe7b1d9a3fc457bafa1650d3da8bd5. The historical local Git
oracle tests are deliberately excluded from this shallow-checkout workflow.

Preparation is first published to the non-trigger
`checkpoint/guide-book-identity-20261106` branch. Only after source review and
coordinator release is the same tree published to the trigger branch
`checkpoint/guide-identity-ci-20261106`. Preparation alone is not a passing test.

Even a successful run proves only these two pure helpers. It does not prove
source_pages native typing, Font/Text page generation, translated Guide refresh,
live screen lifetime, packets or LAN behavior. The existing LAN code is unchanged.
