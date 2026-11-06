# Native language and item display: verified bounded contract

The dedicated public-runner [run 42181415662](https://github.com/sofia-gros/horizon-mods/actions/runs/42181415662) passed on exact commit `0e4d68ea9c0d9cdecf07aed14daf5dacc57710c5`. This result was checked against the decoded job log and its successful final receipt, not only the workflow badge. The source parent is `df0645ec2bdd1063364eb82d85c45ef45d61c876`, with the composed language adapter from `ea7095d8`. No runtime source changes were made by the CI or this documentation checkpoint.

## What passed

- 22 actual Rust cases, one unfiltered execution: internal language loader 8, public Key/Text 6, item display 8. Zero failures, ignored, measured or filtered cases.
- 15 runner regressions and 25 source/preparer checks, including 11 preparation-time TOML conversion cases.
- Cold compilation of official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` crates `horizon-world`, `horizon-client-core`, `horizon-items`, and the actual fixture test target. The original author example type-checks inside this fixture with an explicit `horizon_client` alias; this is not an installed client-library build.
- Official locked dependency fetch, offline fixture resolution with exact package/path identities, source and complete fixture file-set/lock checks before and after, generated OUT_DIR checks, clean tracked/untracked checkouts and bounded process-group teardown.
- Native compilation took 43.925 seconds. The full 22-case execution took 0.202 seconds wall time (libtest reported 0.08 seconds). Minimum free space was 90,025,824,256 bytes, above the required 10 GiB + 128 MiB floor.

The exact named tests, compiler witnesses, phase exits and final receipt are in [the result JSON](native-i18n-ci-result-20261106.json).

## Meaning for mod authors

The new layer uses the one official language selection owner; it adds no competing locale, registry or cache. Standard item/block names use the existing native key/fallback rules. Explicit `minecraft:item_name` translatable components are re-resolved against the active scoped native language without changing the stored component. Custom-name priority, native translation arguments/style, missing-key handling, native format syntax, source and namespace order are covered.

A retained public Key can resolve again after a language change. A previously resolved Text remains a snapshot, which the tests deliberately preserve. A caller must rebuild translated text at the correct UI lifecycle point.

Consumer language files remain `assets/<namespace>/lang/<locale>.json`. Optional TOML is converted to JSON at preparation time by the tool; the runtime loader is not changed to accept TOML.

## What remains open

This proof uses actual native types and exact official Names/styled_name fragments, but it does not instantiate the large Game receiver or draw a window. It does not prove the full package/manifest application, enabled-mod admission, pack/content overlay priority, same-locale resource reload, retained Guide rebuilds, guest-language synchronization, delayed LAN replies, or translation of every mod screen. Content-default propagation to every new ItemStack is also not established. The item component cases are not a general proof of arbitrary NBT/Wire/invalid UTF-16 preservation.

English titles still used as screen or protocol identity were not translated or changed. Their separation from display labels remains a consumer integration task. A workflow pass must not be presented as completion of those remaining paths.

## Source integrity

- Mod checkout seal: `b6dfaa9dbc7faf2dc6e90797b6c143590ed306b9ba413ba5ce0590c65f0b741d`
- Official checkout seal: `3fd9eb59fd89d315f9db87792b584e7f727b9565b000b5814c8b289da83310a1`
- Fixture source/lock/file-set seal: `428b475b4a6fe4ee783e117190a223d639e8d5e30488dffab9c2eee8d028c793`
- Generated output seal: `2e0cccec72c5324ed5fe44c504d31bc7a7e1592914235ca20fa7f1186fd21bff`

This is a development checkpoint, not a main-branch merge or release.
