# Latest chest owned-LOAD diagnostic: rejected

The real official planner rejected the proposed owned `ItemStack` LOAD. This is a recorded capability failure, not a compatible or playable result.

- Run [42222131283](https://github.com/sofia-gros/horizon-mods/actions/runs/42222131283), job `127906499815`, exact source `40a0ee25e73aa7fd641b2bc0fa50a677d457e072`.
- Official Horizon `41f822b659789780f58d32c63ddfb6887ea1b8bf`, Mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`. Official CLI was cold-built; Game/consumer compilation was forbidden afterward.
- Actual plan **exit 1**, monitored **7.270 seconds**. Diagnostic **E_UNSUPPORTED_LOCAL**, `tinkers-chest-pick-color`, `horizon_client::game::block_items::Game<'_>::picked_stack`.
- Exact message: **LOAD modifiers currently require a primitive scalar value type**.
- No accepted plan or selected IR was produced. No apply, Rust/Game type check, complete composition, mandatory full guards, or gameplay test ran.

The GitHub job succeeded only at collecting the bounded diagnostic. All **7 gates / 17 phases** are accounted for: the first 16 phases exited 0, then the one actual plan rejected with exit 1. The remote run passed **24 runner / 16 candidate pure checks**. `diagnostic_complete`, `source_verified`, and `probe_completed` are true; `error` is null. `plan_accepted` is false and `compatibility` remains null for this reduced probe.

## Evidence and limits

The adjacent JSON retains the exact FINAL_RECEIPT, complete fetched console (34,694 bytes; SHA256 `add0dd6526fea3ae533fbe4c93886945430d6547b1597aaab3f54132a59323ad`) and reconstructable hash-verified argv/CPU records. Nanosecond integers are preserved exactly. The raw plan phase was 413 bytes with SHA256 `5dfbb1c06b92ea854b0b4f1788dd095589158cc4a7b053b3ca32b72053425588`; that raw phase file was not separately downloaded.

One exact offline/locked metadata invocation was permitted. Its record witnesses start, not standalone completion. The observer and CLI both exited 1, with two direct-CLI CPU samples. Those samples exclude child processes and do not identify internal planner phases. Final checkout/Git/source/probe/CLI seals passed; minimum free disk was **90,682,253,312 bytes**, minimum observed available memory **13,613,338,624 bytes**. Source seal: `6daddf57b12f9d757e0a1e4adff0583282df8039296aca11ec06850a8f2cd5af`.

## Consequence

The source-derived final `Some(stack)` location and policy-preservation reasoning were insufficient to establish support: the official LOAD kind excludes this non-scalar value. Keep the rejected candidate and finite source tests as evidence, but do not adopt that chest hook into the playtest configuration as a working fix. A distinct official expression/result seam must be verified from its actual contract before another independently reviewed probe. No fallback clone/reconstruction, full-function replacement, guard relaxation, main/PR modification, or automatic retry was performed.
