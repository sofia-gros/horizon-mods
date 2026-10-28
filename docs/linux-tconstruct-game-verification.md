# Linux での TConstruct 実ゲーム検証

2026-10-28 の保存環境では、本体 `/workspace/horizon`、Mod `/workspace/horizon-mods`、本体 `.local/horizon-client/26.3`、`/workspace/.setup/sysroot` を使用した。Minecraft リソース・ワールド・成果物はコミットしない。Horizon `ea51f25aa2e5063a6ca79d4e7277c8df090f9d59` と horizon-mods `ad28aa2222a71bcc391c10ca5710176845bf861b` の組で確認した。

```bash
export PATH=/workspace/.cargo/bin:$PATH
export RUSTUP_HOME=/workspace/.rustup CARGO_HOME=/workspace/.cargo
cd /workspace/horizon
cargo build --profile fast -p horizon-client --no-default-features --features mods
cargo build -p cargo-horizon-mod
cd "/workspace/horizon-mods/Tinkers' onstruct"
/workspace/horizon/target/debug/cargo-horizon-mod horizon-mod build
cd "/workspace/horizon-mods/Tinkers' onstruct/server-tools"
/workspace/horizon/target/debug/cargo-horizon-mod horizon-mod test
/workspace/horizon/target/debug/cargo-horizon-mod horizon-mod check target/horizon-mod/tconstruct_tools.hmod
```

ヘッドレス環境では一時 X server と software Vulkan を使う。空いている display 番号を選び、終了時に自分の Xvfb を停止する。

```bash
/workspace/.setup/sysroot/usr/bin/Xvfb :178 -screen 0 1280x720x24 -nolisten tcp -ac &
xvfb_pid=$!
export DISPLAY=:178
export LD_LIBRARY_PATH=/workspace/.setup/sysroot/usr/lib/x86_64-linux-gnu
export VK_DRIVER_FILES=/workspace/.setup/sysroot/usr/share/vulkan/icd.d/lvp_icd.json
export PYTHONDONTWRITEBYTECODE=1
cd /workspace/horizon-mods
python3 tools/probe_tconstruct_flint_repair.py
python3 tools/probe_tconstruct_flint_repair.py --in-frame
kill "$xvfb_pid"
```

両方式で、Creative で作った破損試験個体を有効化し、Survival で火打石を2個消費して修理、無傷での再操作・保存復帰を確認した。`Game` ヘルパーは実行ごとの一時 world・endpoint・ログを作り、`HORIZON_HOME`、`HORIZON_LAUNCHER_HOME`、`XDG_DATA_HOME`、`XDG_CONFIG_HOME` を分離して self-update と audio を無効にする。ログと JSON report は `.local/repair-tests/` に出る。専用サーバー、複数プレイヤー、Station UI はこの試験に含まれない。
