# Releasing v0.5.0

Only create and push `v0.5.0` after the final source changes, non-gameplay checks, and issue #379 residuals have been reviewed. The tag starts the release workflow. Do not use it for an intermediate build: publication is automatic after validation succeeds. Gameplay testing is deferred to v1.0.0; do not present it as completed.

The workflow runs the Mixins validation on the tagged source, using the same checkout's PlutoLib and the Horizon and Mixins revisions in `tconstruct/compatibility.json`. It then builds `tconstruct.hmod`, `tconstruct.hnmod`, and `plutolib.hnmod` from that exact commit. The validator checks archive integrity, package identities, Tinkers version against the tag, and the native compatibility pins. `SHA256SUMS` accompanies the three packages. PlutoLib is a native-only bundle, with its own independent component version; there is no empty PlutoLib content mod.

Before tagging, update the release notes with the final known limitations and ensure the Tinkers content and native manifest versions match the tag. Inspect Actions after the tag push and verify all three files and checksums on the release. If validation or publishing fails, investigate the run; do not move an already published tag. This workflow is deliberately limited to `v0.5.0` so a later tag cannot accidentally use these notes.
