# Windows BuildOnly first attempt

Exact controller 44f0f7af1ab966bbcb03e14621dd63bac104fb3c, ordinary input 9e032ef5b013487ea08b997eb08df84825fb09ba. [Run42334259441](https://github.com/sofia-gros/horizon-mods/actions/runs/42334259441) stopped at the existing-runner Rustup version admission on 2026-11-07 at03:30:19Z.

The 17 pure tests passed on Windows. All six child phases exited0, all owned Jobs were empty, and final source/tool seals passed. The attempt remains passed=false/verified=true. Rustup --version produced158bytes and exit0, but the controller required a1.29.2 first line. The actual image manifest reports1.29.3; raw version text was not retained in the receipt, so its exact first line and stream ordering are not established.

Observed minima: C:33,450,332,160bytes; D:157,743,054,848bytes; available physical RAM14,159,241,216bytes. This attempt used windows-2025-vs2026 image20261103.298.1 rather than the earlier source-reviewed Windows2025 image. The image's exact documentation is linked in the JSON summary.

Toolchain installation, official source/Cargo fetches, fresh plan, guard/apply, compilation, imports and help were not reached. No fallback, extra download or automatic retry occurred. The full console is retained locally; this publication contains only the small result summary and hashes.
