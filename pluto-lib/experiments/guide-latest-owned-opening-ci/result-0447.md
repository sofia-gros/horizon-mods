# Latest Guide ownership attempt: fixture compile stopped

Exact head `0447b0bab69102dbd5456617a6b6542f8bada4db`, run
[42209312310](https://github.com/sofia-gros/horizon-mods/actions/runs/42209312310),
ended **failure**. The fresh complete official f4dc game subtree inventory
passed: 79 book accesses and 26 attachment assignment/take sites, with all 21
pinned source files. This is a spelling inventory, not alias or lifecycle proof.

The next phase, std-only fixture compilation, failed with E0282 at
`guide-owned-opening/fixture.rs:96`. The local `Rc<RefCell<RetainedBook<_>>>`
needs an explicit `Value` type before a destructor callback accesses its fields.
No owned-storage test executed. The 13-file experimental Game transformation,
metadata comparison and actual Game/BookScreen compiler gates were not reached.

The original final receipt is preserved verbatim as JSON data alongside this
file, including its incorrect summarized `known_source_files: 19`. That field
was an old hardcoded count; the strictly checked complete generation map contains
21. The proposed correction derives the count from the actual checked source
map. The false `passed` value is unchanged. `verified: true` denotes final source
seal verification only, not a successful Rust or host gate.

The complete compact console is 21,082 UTF-8 bytes, SHA-256
`d139cbd54b075c90c32d1b6585727edcd22bb2306999ecf69d012fecc0043c0d`.
Raw phase files were not retrieved; their hashes and the bounded compiler
diagnostic remain in this console. Minimum free space was 91,088,003,072 bytes.

A separate test-only type annotation and receipt-count regression are proposed.
No automatic retry, ordinary manifest change or existing LAN removal occurred.
Actual Client/UI, opening permission, retained-screen lifetime, language reload
and delayed LAN replies remain outside the evidence.
