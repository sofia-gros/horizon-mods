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

## 汎用 GUI と制作・組立

上記の本体と TConstruct のビルドに加え、`examples/gui-storage-fixture` とその `server` でも `cargo-horizon-mod horizon-mod build` を実行する。上記と同じ表示環境で、`tools/probe_tconstruct_crafting_gui.py`、`tools/probe_tconstruct_station_repair.py`、`tools/probe_generic_machine_gui.py`、`tools/probe_generic_storage_gui.py` を通常方式と `--in-frame` の両方で実行する。`tools/probe_generic_storage_players.py` は LAN の二人が同じ収納を開く競合を確認する。`tools/probe_tconstruct_table_recipes.py` は作業台でのテーブルと型紙のレシピを確認する。実行ごとの world とログは `.local` に隔離される。

## Station のレイアウト・名前・同時取得

Station の入力は保存される収納で、組立と修理のボタンを選ぶ。`tools/probe_tconstruct_crafting_gui.py --scale 1` と `--scale 2 --in-frame` は制作・名前・Enter/Escape・中断再開・保存復帰を確認し、`tools/probe_tconstruct_station_repair.py` は wood/flint 修理、無効材料とピッケルの単一部品交換を確認する。`tools/probe_tconstruct_station_players.py` は2人の異なる名前と同一結果の同時取得を検査する。GUIの詳細な原典との差と未対応範囲は `docs/tconstruct-crafting-gui.md` に記す。

## 複数slot修理

`tools/probe_tconstruct_multislot_repair.py` と `--in-frame` は、火打石を分割した修理、間の不正材料による拒否と復帰、収納の保存・再読込、2人の同時取得とslot順の一度だけの消費を実ゲームで確認する。対応範囲はwood/flint pickaxeで、repair kit・他素材・他ツール・未対応modifierは含まれない。
