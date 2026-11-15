# Horizon Mods

[English](README.md)

**PlutoLib**は公式Horizon向けの独立した汎用APIライブラリです。**Tinkers**はPlutoLibの公開APIだけを使い、直接注入は持ちません。

## 現状

新しい`tconstruct/`は、直接注入0の空の起動入口です。**新製品へのゲーム機能の移植・ゲーム動作確認はまだ行っていません。遊べるリリースではありません。** PlutoLibの既存実装も、今回の整理だけで検証済みにはなりません。

旧Tinkersは未コミットのコード・資産を含めて`legacy/2026-11-14/tconstruct/`へ退避しました。新しいパッケージには含めません。旧スクリーンショット・棚卸しの状態は、新製品の動作実績ではありません。既存CIは保持しており、新構成との整合は次の作業です。

```bash
python tools/port.py next
```

```bash
python tools/port.py check
```

[作業規則](AGENTS.md) · [移植計画](docs/roadmap.md) · [設計](docs/architecture.md) · [実行方法](docs/development.md) · [PlutoLib API](pluto-lib/docs/api/index.md)

[検証済み開発ビルドの起動](docs/development.md#verified-development-launch)は、ソースと対応するバイナリを明示して使い、毎回新しい隔離ホームを作ります。旧ビルドへのフォールバックはありません。遊べるTinkersリリースではありません。

成果は原作どおりに動くコードです。テスト・文書はそのために必要な最小限に絞ります。旧資産を再利用するときは原作との一致とライセンスを確認します。
