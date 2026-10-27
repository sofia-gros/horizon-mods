# M3b4 採掘耐久の検証

2026-10-27。Horizon `b4bfb44a7af1f20e944f61dba6403d20ccd890d1` の同期採掘 hook と、同コミットに SDK を固定した server-tools を使用。対象は基本 pickaxe の採掘耐久であり、modifier、修理、攻撃時耐久、専用の耐久バーはこの確認の範囲外です。

原作の基本ルールに合わせ、成功した採掘で硬度が 0 でない場合は耐久を 1 消費し、硬度 0 または broken の場合は追加消費しません。Creative では消費しません。本体は破壊時のプレイヤー・スロット・アイテムを確認して同期的に Mod を呼び、対象以外のプレイヤーやアイテムへの変更を拒否します。

- Rust 単体テスト、Java 原作との比較、Wasm sandbox と package 検査を実施。hook の別 owner / 別 player、エラー・trap の取消を確認。
- integrated server と `--in-frame` の実ゲームで、それぞれ Creative、硬度 0、通常採掘、耐久限界、broken、持ち替え、vanilla アイテム、保存・再起動を確認。二人のプレイヤーを接続し、各個体の独立した耐久更新も確認。これらの一連の自動試験では同じ SDK ソースをローカル参照した Wasm を使用。
- 固定 SDK から再ビルドした package でも `--in-frame` の実ゲームで flint / wood / wood のピッケルが鉄鉱石を破壊し、鉄の原石 1 個がドロップ、ToolState の damage が 0 → 1（耐久上限 85）となることを確認。[前後の画像](images/tconstruct-m3b4/README.md)。見た目は木のピッケルであり、ゲーム内の通常の耐久バーはコア状態を表示しません。
- 同一 server tick に複数採掘が重なる場合と専用サーバーでの新 hook は未検証。modifier や特殊採掘を含む原作全体の互換性は未完了。

試験用の分離ワールド、ログと詳細 report は `.local/` に生成され、Git には含めません。再実行用スクリプトは `tools/probe_horizon_mining_wear.py`、`tools/probe_horizon_mining_players.py`、`tools/capture_tconstruct_mining.py` です。実行には準備済み Minecraft 26.3 リソース、Horizon / horizon-ctl、ビルド済み Mod package と仮想ディスプレイまたは Vulkan 対応画面が必要です。各スクリプトの `--help` を参照してください。
