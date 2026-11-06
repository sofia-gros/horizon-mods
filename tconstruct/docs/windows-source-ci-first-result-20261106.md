# First Windows source-only run: partial failure preserved

Exact head c93675e0921c9a2b01bf612a4c6ce4aa45da7f55, run42230840423, job127935995641, failed2026-11-06T14:06:20Z. [Run](https://github.com/sofia-gros/horizon-mods/actions/runs/42230840423).

The actual standard runner was Windows Server2025, Python3.12.10 x64, not a Windows11 playtest. PowerShell Parser.ParseFile passed in4.64s without executing the launch script. The latest20 suite ran19 successful cases and1 failed stage-source hash case. Historical12 and synthetic bundle11 were not reached. Both reached phases completed Job Object assignment and empty teardown. The final receipt is passed=false, verified=true: all6771 source files had matching before/after seals, not all checks passed.

The failure is an encoding error in the test: Path.read_text() used the Windows locale to decode UTF-8 source containing a curly apostrophe. Re-encoding that mojibake produced SHA d15ecca5ac8051c2419ca269ed4a952b580536b1e02c55ac7734e14776e7e260. Independently decoding the same pinned bytes as cp1252 and re-encoding UTF-8 exactly reproduces the observed value. The raw source still has the expected c2d14a49bfc55caadb8159441e87ee6a21203586bf7a725b4648ecd653166092 hash.

The proposed correction hashes raw bytes and explicitly decodes UTF-8 for the textual inverse, and adds the exact CP1252 reproduction assertion in the existing test. Test names/counts, inverse expected hash, launch source, guarded stage, Job Object/resource bounds and strict parser remain unchanged. All43 actual merged pure outputs pass the unchanged strict validator on Linux, within11 preparatory regressions. A Windows retry is not yet executed or authorized by this source record.

The companion JSON preserves the full actual FINAL_RECEIPT, partial result and SHA/size of decoded tool-console UTF-8 content. Raw console is not published here; native phase logs remain represented by their original runner hashes. No Cargo, Rust toolchain, game/engine, assets or installation ran. This is not package generation, full composition/guard, build, startup, GPU or playability proof. Normal checkout action setup used its own temporary Git configuration; the new controller performed no permission/system configuration change.

A scoped audit of all three suites also found eight remaining implicit read_text calls, with currently ASCII-only referenced source/JSON. These now explicitly use UTF-8; the synthetic bundle suite has no text reads. An AST assertion inside an existing test checks every read_text call in the three suites. Names/counts remain20/12/11.
