# Compatibility

## Current development target

TConstruct and the separate PlutoLib native bundle must use the same pinned
Horizon and Mixins revisions:

- **Horizon:** [`86a60b1b`](https://github.com/horizon-rust/horizon/commit/86a60b1b9670abba865e079b12d6f978c73d8cba).
- **Mixins:** [`ddd9700b`](https://github.com/horizon-rust/horizon-mixins/commit/ddd9700b21997c95d95ac6ee5ecafe3830cf622a).
- **PlutoLib:** build the independent `.hnmod` from the same horizon-mods checkout as TConstruct.
- **Source TConstruct data:** [`f4e77d43`](https://github.com/SlimeKnights/TinkersConstruct/commit/f4e77d43f39ba1047bf93ae816b7e1beaab514c2).

The authoritative pins are in [`compatibility.json`](../compatibility.json).
This target is being validated in [PR #103](https://github.com/sofia-gros/horizon-mods/pull/103).
Changing these pins does not establish build or gameplay compatibility.

**Current blocker:** the Mixins printer converts an existing host float range
`0. ..=1.` into invalid Rust `0...=1.`. This reproduces independently with the
pinned parser/printer dependencies. No generated-file patch is part of this
candidate; it must remain unmerged until the official combined output is valid.

## What has been verified

The earlier combined candidate on Horizon `0c7b5b61` and Mixins `ddd9700b`
passed the official combined apply, a Linux client build, and representative
Creative, Station, casting and save/restart checks. See
[PR #354](https://github.com/sofia-gros/horizon-mods/pull/354) and
[the main integration record](main-integration-354.md) for the tested code and
subsequent integration changes. Those checks do not validate the newer host
above or every feature in the current main branch.

The integrated Lantern Basin guard has focused source tests; its corrected
in-game behavior still needs confirmation. Windows and complete gameplay
coverage also remain unverified.

## Verify an integrated build

Use [Issue #365](https://github.com/sofia-gros/horizon-mods/issues/365) to record
the tested main commit and platform, then check related features together in
one build. A merged PR or a passing unit test is not an in-game pass.

LAN-only limitations are tracked separately, including
[guest Overwield mining speed](https://github.com/sofia-gros/horizon-mods/issues/363).
They do not disable unrelated single-player features. Position-dependent
foliage tint remains deferred; island and vegetation hooks remain enabled.

Older compatibility evidence is retained in [host upgrade notes](host-upgrade.md).
Use it only for its recorded revisions and scope.
