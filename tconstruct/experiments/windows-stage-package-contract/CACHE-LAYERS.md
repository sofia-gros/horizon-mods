# Historical cache test layer repair

The original cache suite had two errors on both the c1b54 baseline and 061911: it compared today's PowerShell entry directly with the pre-formatter/latest-entry inverse hash. The runtime cache tests were not failing. This follow-up changes only test/witness/documentation files.

legacy_script now first verifies and reverses the existing exact link-only layer (9defc8c8 to f456a1ff), then reverses the four formatter additions (f456a1ff to 1c4271db), before running the original latest-entry and original cache inverses. The formatter inverse must reconstruct the already recorded whole-file 1c4271db hash, not a newly substituted expected result. The existing link/latest/cache witness bytes and all old test assertions remain unchanged. PowerShell and every runtime/helper byte remain unchanged.

The original 12 cache tests now pass, including both previously failing historical checks. Three added tests verify the exact old test body and three old witness hashes, reject current-byte/newline drift, missing edits, wrong layer order and wrong prior hashes, and require both new layers rather than silently falling back if either is absent. These are pure source/cache tests. They do not execute the PowerShell source, including its retained historical resource strings.

The local selected sparse test collection passes 92 tests (the prior 77 plus cache12 and layer3). This is not an all-repository test discovery or an actual Windows/formatter/build/guard-composition success. The previous two-error result remains historical evidence in the earlier source/caller verification files, not rewritten as a past success.
