# Original PlutoLib source history archive

This checkpoint preserves the original MOD repository Git objects through local commit `b19b35a10b96e398ca8dabcb46b200d88950edaf`. It is an archive for recovery, not a reconstruction of the ordinary GitHub PR commit graph and not a claim that all consumer integration is complete.

## Scope and verification

- Repository: `sofia-gros/horizon-mods` only.
- Required existing main prerequisite: `08afa992d5091e03360f3473ea93fd17aa793987`.
- Original source branch: `refs/heads/feat/plutolib-shared-api-recovery-20261105`.
- Original commits after that prerequisite: 203. The manifest lists every commit.
- Exact restored tip tree: `3ee126db87fe4becdc9fb20c5c10c81c21aafb15`.
- Bundle: `pluto-shared-source-history-b19b35a1.bundle`, 1025355 bytes.
- SHA-256: `86b5b94597cbce5ca3840ebc2d19e0c46a9e145492544c13508f865502ec9793`.
- Imported into a new bare repository. All 2,377 required new objects were found in the imported pack, and all 203 original commit objects matched byte-for-byte without using alternates.
- Only committed MOD history in this range is included. Later unmerged worker research branches, untracked files, generated stages, caches, credentials, and execution logs are not included.
- Every newly included blob/path was screened for generated/private workspace paths and common private-key/token/assigned-secret patterns, with no hits. This is a bounded inspection, not a universal secret-detection guarantee.

## Restore

Use a clone of the MOD repository that contains the required main prerequisite. Download the bundle and verify its SHA-256 against the manifest, then run:

```sh
git bundle verify pluto-shared-source-history-b19b35a1.bundle
git fetch ./pluto-shared-source-history-b19b35a1.bundle \
  refs/heads/feat/plutolib-shared-api-recovery-20261105:refs/heads/restored/plutolib-source-20261106
git rev-parse restored/plutolib-source-20261106
git rev-parse restored/plutolib-source-20261106^{tree}
```

The last two values must equal the tip and tree above. The bundle requires the stated public main base; it is intentionally not a full copy of unrelated repository history.

## Publication and readiness

The connected GitHub account was authenticated and successfully published the source checkpoints. The execution environment's separate git CLI could not obtain a username through its credential/askpass path. No authentication settings or credentials were changed.

PR #944 preserves the reviewed v13 source snapshot as a normal snapshot commit. The v14 checkpoint at `43411ddb6e5be857db96740150319a7c2e83eed1` preserves the later exact tree. This bundle additionally retains the original 203-commit development history and metadata for recovery. It does not change either PR's ordinary ancestry or merge status.

Official-library v14 plan, twelve guards, apply, and client library/binary checks passed. Full TConstruct consumer composition, live guide response ownership, automatic crafting integration, and actual terrain CPU/GPU execution remain separately limited or unfinished. Main has not been merged.
