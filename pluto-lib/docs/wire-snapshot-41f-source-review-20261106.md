# Wire snapshot:41f source review only

The first integrated plan was accepted, but the unchanged Wire guard refused the whole item_data.rs file hash. This is a demonstrated source-pin mismatch, not a demonstrated private-Wire layout break.

- Existing metadata pins624c SHA59a36417; exact41f is744c7ae0 (76,150B).
- Wire declaration, tuple fields, constructor, patch reader and PartialEq region are byte-identical. The earlier prefix containing StackData is also unchanged.
- Existing Pluto wire-read.rs remains4d245759 and only borrows the retained baseline plus patch. It does not project current state or re-encode data.
- Changed runtime code adds numeric boolean handling/foil use, two borrowed accessors, default lodestone/hive saved-component normalization and numeric damage decoding. These are real native behavior changes outside the borrowed Wire seam and are not claimed equivalent.

The adjacent JSON retains the exact runtime diff, old/new hashes and verified unchanged regions. Official41f raw bytes were independently fetched and compared. No runtime, guard or metadata has been edited. The complete latest plan's3module owners, exact sources/visibility and absence of additional item_data injections/states must still be checked from preserved raw plan. Latest native typing and broader serialization behavior are not proved by this source review.
