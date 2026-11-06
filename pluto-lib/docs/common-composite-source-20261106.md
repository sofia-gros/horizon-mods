# Combined PlutoLib source checkpoint

Status: source composition and reproducible packaging only. This complete combined source has not yet passed official plan/apply, Rust typing or native tests. Independent historical successes are not a combined compatibility result. Main and all TConstruct consumers remain unchanged.

## Exact layers

- Base: author API proof `f438f340cda4c16f7d374fbb61ed5c528cecec59`. The complete inventory API (33 files, historical 147 tests) and public API (11 files, historical 58 tests) remain byte-identical, including Java/menu exports, Guide registration, recipe presentation builders, inventory aliases and fluid declarations.
- Rendering: seven native implementation files from `43f174273cd5f1b9cf3acd1d26abab8838025510`. The public fluid API already contains this implementation plus only the two Declaration export lines; removing those lines reproduces the rendering cutoff's fluid.rs exactly.
- Language: the two unchanged installed modules and preparation tool from `ea7095d80b67ab8659b26b3e0e32a92d77de2cae`.
- Guide Font adapter: unchanged guide-layout.rs from `c9bc56eeea60a6caf88de0b8b1055cc33a8155e0`. No TConstruct draw, click, packet, title, request or lifetime code is imported.

The only combined manifest edit appends the two language module blocks to the authoring base. Removing the exact recipe-presentation and two language blocks reconstructs the original v15 manifest bytes. There are no new injections or states: the complete bundle has two crates, 69 modules, 34 injections and three states, each known owner appearing once.

The terrain source-check ledger's whole-manifest pin is deliberately updated to this new manifest. A new inverse test proves the exact three additive blocks and original manifest hash. The old immutable terrain proof is not rewritten or represented as a proof of this composition.

## Source validation performed

- 19 existing language/TOML source checks passed.
- 23 terrain source/package checks passed.
- Four new composition checks passed: complete API file sets, inverse manifest, all 168 runtime-source hashes, and actual package ownership.
- The native archive was produced twice with the unchanged packager and was byte-identical. SHA-256: `4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d`.

Preparation history is retained in the JSON. The first terrain source run lacked its source ledger, and the original whole-manifest assertion subsequently rejected the known additive modules. Exact ledger materialization and the narrowly pinned composition check resolved these preparation issues. No runtime body was edited to make a check pass.

## Required next gates

1. Independent source review of layer identities, complete API sets, manifest ownership and exact package inputs.
2. One new official whole-bundle plan and all 12 guards, then apply to the pristine official source and actual host library/binary typing. The recipe presentation support module is an explicit host-type gap, not implicitly approved by its pure API tests.
3. Separate exact native groups for installed language, terrain facade/material, Guide Font and recipe presentation, with their own claims. Preserve the historical pure API tests and examples on the new cutoff.

This checkpoint does not activate new Game authority, contact effects, item-name default/removal hooks, retained-book language refresh, LAN, live UI drawing, resource-pack overlay correction or GPU work. It preserves the two-layer direction: public wrappers/builders/reexports delegate to the same typed internal owners, while native host integration remains explicit and separately verified.
