# Local Rust syntax verification, 7 November 2026

Official Rust 1.99.1 Linux GNU Rustfmt was used as the Rust parser, not a custom
parser. Its actual version was `rustfmt 1.10.0-stable (58cf8f9c35 2026-10-13)`.
The official manifest matched its SHA256 sidecar. Only Rustfmt and the required
Rustc runtime libraries were placed in an isolated temporary directory.

Eight syntax cases matched their expected results: five valid/repaired inputs
returned 0, and three `0...=1.` cases returned 1 with the expected `...` syntax
error. Each took 0.022–0.043 seconds. This includes the full official 72,769-byte
start.rs, an artificial bad copy, and the exact repair restoring its original
SHA256 ccb0e6ac41d7af0cd893ca5cfc4f659ca9c0b8a2ad17f5148fd70451ed38352d.
An absent-module sentinel confirmed stdin did not traverse child files.

The actual proposed stage gate was then run on those three full-file variants:
valid 0, artificial bad 1, repaired 0. It uses an empty explicit config and
`--edition 2024 --emit stdout` with stdin; it does not write the source and does
not use formatting-sensitive `--check`. This proves syntax behavior, not types,
linking, game execution, or acceptance of an actual generated stage.

The failed Windows run 42334764929 has no downloadable artifacts, so its full
generated file could not be recovered. That limitation remains explicit.

Preparation was not initially successful. Python XZ extraction terminated with
137; a bounded XZ decoder rejected its first memory limit, and a later native
stream ended unexpectedly. Their causes are not called syntax failures. The
small musl formatter was inspected but not run because it required a different
loader/runtime. The final successful path streamed the same official GNU gzip,
verified all 141,445,153 archive bytes and SHA256, and extracted only 366,408,442
runtime bytes in 10.508 seconds. Only this attempt's recorded, reproducible
failed temporary binaries/archive were removed; source and proof files remain.

Workspace free space stayed above 10 GiB, and the independent temporary volume
stayed above 4 GiB. The successful stream's minima were 10,871,975,936 bytes
workspace, 4,785,344,512 bytes temporary space and 1,470,279,680 bytes available
physical RAM. Parser children had 1 GiB virtual-address-space, 20 CPU seconds,
30 wall seconds and 1 MiB output-file limits. These are stop limits, not a claim
of a container-wide memory cap. Exact input, binary, archive, output and runtime
hashes are in the companion JSON.
