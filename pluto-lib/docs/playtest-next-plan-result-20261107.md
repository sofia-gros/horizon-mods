# Latest integrated plan diagnostic

Run [42328100345](https://github.com/sofia-gros/horizon-mods/actions/runs/42328100345), job 128268027000, completed on 2026-11-07 at 02:24:49 UTC. Controller 7082868bcbf37ed3eabd32f40ddf0cf08a4ee30c tested integration e9073648b3c4cf4d49f27bc2e1ed60997f430586 on official Horizon41f822b / Mixins5a64af93. Later integration changes are not part of this result.

The official full plan was accepted in 460.676 seconds: 320 injections, 329 modules, 111 states, 3 bridges, 3 crates and 1 data entry. Raw plan: 6,431,102 bytes, SHA256 dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b.

All 12 guards ran. Eight passed, including Wire, both terrain guards and damage equipment. Four refused: client payload (unreviewed plan/Index observation), recipe placement (dependency module identity), crafting match (unobserved native fingerprint), and cloud insertion (unreviewed placement observation). Compatibility remains false.

The separate Index tool compiled against actual official core and successfully identified native_client_packet, handle_packet and poll_remote. The two known identity controls matched. poll_remote is node [44,41], fingerprint e1b7b4869ed3ffcebf6acf7e70cd9acfd66acfbe2363e46abf901a6489a80aac. These source identities do not prove caller composition or host typing.

Artifact 13556955569 contains the original four bounded files and was downloaded and validated by exact filename, size and SHA256. Its MAIN receipt matched the console MAIN JSON without integer loss. The main artifact was uploaded before Index and is separate from its receipt. The 930-byte Index stdout was not recovered directly; its size/hash are phase witnesses, while the result object is retained in the small JSON summary.

No apply, Game build, installed type check or game launch was performed. No guard was relaxed. Job success records a completed diagnostic, not a playable build.

This report intentionally contains only bounded project results and hashes, not raw logs, environment data or full input maps.
