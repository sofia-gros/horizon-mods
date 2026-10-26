# Tinkers’ ConstructのHorizon移植調査

後続の [詳細実装調査](tconstruct-implementation-plan.md) で、ユーザー指定の本体フォークと `Tinkers' onstruct/` の配置、および具体的な追加箇所を確定しています。以下の配置案より後続調査を優先してください。

調査日: **2026-10-26（日本時間）**。
対象の公開ソース・生成済みデータ・テスト・公式文書を確認した、移植着手前の調査です。
今回は実装、Gradle / Cargoビルド、ゲーム起動、性能測定を行っていません。

## 結論と推奨する出発点

**Tinkers’ Constructは移植対象として適しています。** コード・テクスチャ・バイナリはMITと案内され、素材やツール定義がデータとして分離され、性能計算・状態管理に既存テストもあります。
ただし、現行Horizonの公開Mod APIだけで完全版を動かすことはできません。
素材・性能計算のRust実装を先行させ、並行して必要なHorizonのAPIを追加する方式が妥当です。

最初の遊べる範囲は、**少数素材 → 部品作成 → つるはし組み立て → 採掘・破損 → 修理**を提案します。
溶融炉・合金・鋳造・全Modifier・防具・遠距離武器・ワールド生成は、その後に段階的に扱います。
最初の試作でも、未対応の能力を黙って無視して完全互換と見なさず、対応範囲を明示します。

根拠: [上流README](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/README.md)、[ToolStackと性能再計算](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt/ToolStack.java)、[Horizon WIT](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mod-sdk/wit/horizon.wit)。

## 調査対象を固定する

| プロジェクト | 対象 |
| --- | --- |
| Tinkers’ Construct | 既定ブランチ `1.20.1`、コミット `f4e77d43f39ba1047bf93ae816b7e1beaab514c2` |
| 原作の環境 | `mod_version=3.12.1`、Minecraft 1.20.1、Forge 47.4.0、Java 17 |
| Mantle依存 | 原作の指定は1.11.115、許容範囲は `[1.11.113,)` |
| Mantle補助調査 | ブランチ `1.20`、コミット `021a4f5043a9d2f78604032427b9e98c3171c8b1`。1.20.1向け。指定配布物1.11.115と同一ソースであることまでは確認していない |
| Horizon | 調査時のHEAD `45bc45b3967f06ab56173fe2b13892004e717e35`。公開manifest APIは0.1 / 0.2 |

バージョン指定がないため、Tinkers’ Construct 3の既定ブランチを採用しました。1.12.2のTinkers’ Construct 2とは仕様を混ぜません。
上記の3.12.1は調査コミットのプロパティ値であり、特定の公開済みjarと完全に一致することを保証する表記ではありません。
差分検証用の原作環境を用意する段階で、配布物・ソース・Mantleの正確な対応とハッシュを揃えます。

根拠: [バージョン指定](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/gradle.properties)、[Java・ビルド依存](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/build.gradle)、[Horizon manifest](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mods/src/manifest.rs)。

## ライセンスと依存

Tinkers’ ConstructのREADMEはコード・テクスチャ・バイナリをMITと明記し、例外のライセンスは該当箇所に記すとしています。
MIT対象部分を再利用する際は、著作権表示とライセンス本文を保持します。
MantleもMITです。今回の確認でCreateのようなassets全体をARRにする記述は見つかっていません。
ただし、個別の外部素材・追加依存・別ライセンス表示の全件監査は未実施です。ファイルを取り込む際に出典と条件を記録します。

ForgeとMantleは原作実行時の必須依存です。JEIとJson Thingsはmetadata上では任意依存であり、初期移植の必須条件にはしません。
Gradleの開発用runtime依存や互換確認用Modを、そのまま移植先の必須依存として扱いません。

Mantleは登録、JSONローダー、スロット・画面、流体、ネットワーク、ガイドブックなどを支えます。
Rust版でMantle全体を別Modとして移植する必要はなく、使う機能をHorizonの共通APIとMod内ライブラリへ置き換える設計が考えられます。
これは設計案であり、MantleのJavaバイナリをHorizonで読み込めるという意味ではありません。

根拠: [TConstruct LICENSE](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/LICENSE)、[依存metadata](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/META-INF/mods.toml)、[Mantle LICENSE](https://github.com/SlimeKnights/Mantle/blob/021a4f5043a9d2f78604032427b9e98c3171c8b1/LICENSE)、[Mantleの実装](https://github.com/SlimeKnights/Mantle/tree/021a4f5043a9d2f78604032427b9e98c3171c8b1/src/main/java/slimeknights/mantle)。

## ソースの構成と移植単位

主なJava実装は `src/main/java/slimeknights/tconstruct/` にあります。

| 領域 | 主な責務 | 移植の扱い |
| --- | --- | --- |
| `library/materials` | 素材ID、定義、部品別stats、traits、読み込み・同期 | 先行してRustでデータ構造と検証を作る |
| `library/tools/definition`、`library/tools/nbt` | ツール定義、部品構成、個体状態、stats再構築 | 最初の計算コア |
| `library/modifiers` | Modifierの登録、module、hook、条件処理 | 小さく対応集合を定めて拡張 |
| `tools` | 各ツール、素材stats、Modifier・防具などの具体実装 | 最初はつるはしに限定 |
| `tables` | Part Builder、Tinker Station、収納など | 独自Block Entity・menu・同期が必要 |
| `smeltery`、`fluids` | Melter、Smeltery、燃料、液体、鋳造、合金 | 第2の大きな縦切り実装 |
| `library/client`、各moduleのclient | 素材別モデル、tooltip、画面、Block Entity描画 | Javaのrendererを直接流用せず描画方式を実装 |
| `world`、`gadgets`、`shared` | 資源・構造物・生物・装備・共通登録など | 初期ツール体験から切り離して段階導入 |
| `plugin` | 他Modとの連携 | 対象機能と相手Modが揃ってから |

Forgeの登録・event bus・capability・Minecraftクラスへの依存が広く存在します。
Javaコードの機械的なRust変換だけでは移植にならず、ゲームとの接続点を置き換える作業が必要です。
入口: [TConstruct.java](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/TConstruct.java)。

### 規模の目安

調査コミットのファイルを直接集計しました。件数は機能数・移植工数・実行時の有効件数ではありません。

| 対象 | 件数 |
| --- | ---: |
| `src/main/java` の `.java` | 1,938 |
| `src/generated/resources` の `.json` | 7,141 |
| `src/main/resources` の `.json` | 6,288 |
| generatedの `data/tconstruct/recipes` のJSON | 3,067 |
| generatedの素材definition / stats / traitsのJSON | 104 / 99 / 99 |
| generatedのtool definition / modifierのJSON | 45 / 308 |

mainとgeneratedは別の入力ルートです。パスの重複、条件付きデータ、任意Mod向け互換データ、参照先の存在を解決してから移植対象を決定します。
例えば素材definitionの104件を「全104素材が単独環境で必ず利用可能」とは扱いません。

レシピJSONのトップレベル `type` を読み取ると、`tconstruct:melting` 502件、`casting_table` 439件、`material` 217件、`casting_basin` 142件などがあり、通常のcrafting以外が多数あります。
`minecraft:crafting_shaped` は247件、`crafting_shapeless` は63件。これらも結果IDやingredient、Forge拡張が対応するかを確認する必要があります。
`forge:conditional` は66件あり、その内側のレシピを有効条件に応じて扱う必要があります。

再集計は対象コミットで各パスの拡張子を再帰集計し、recipeはJSONをparseしてトップレベル `type` でgroup化します。
大文字・小文字が異なるレシピキーを保持できるparserを使います。PowerShellでは `ConvertFrom-Json -AsHashtable` を使用しました。

## 最重要の実装ポイント

### 素材・部品・ツール状態

素材は `tinkering/materials/definition`・`stats`・`traits` に分かれます。
ツール定義は、部品、基本性能、Modifier slot、初期trait、tool action、有効な採掘対象、範囲などのmoduleから構成されます。
つるはしの定義でも、頭・柄・bindingの3部品、`pierce` trait、採掘tagなどを含みます。

`ToolStack` は次を区別します。

- 永続する素材配列、アップグレード、Modifier固有データ、破損状態。
- 再構築される性能、倍率、合成されたModifier、volatile data。
- vanillaのdamage等との接続。

stats再計算は、定義・registry・tagの読み込み完了を確認し、upgradeとtraitの合成 → volatile data → statsと倍率 → raw dataの更新、という順序を持ちます。
単純な「素材性能を全部足す」実装では一致しません。HeadとHandleでもstatsへの適用方法が異なります。
floatの精度、丸め、既定値、traitの重複・順序、再読み込み時のキャッシュを検証対象にします。

参照: [つるはし定義](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/tool_definitions/pickaxe.json)、[素材statsの適用](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/definition/module/material/MaterialStatsModule.java)、[ToolStack](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/nbt/ToolStack.java)。

### Modifierは数値補正だけではない

`ModifierHooks` はstatsだけでなく、採掘速度、ブロック破壊、耐久、装備変更、攻撃、被弾、使用中処理、飛び道具などに関与します。
初期対応では、ツールや素材が持つ初期traitも含めて依存するhookを列挙します。
「Modifierを後回し」にしても、選んだ素材のtraitを黙って落とすと原作の性能と一致しません。
未対応module・Modifierを検出したら、非対応として診断できるloaderにします。

参照: [ModifierHooks](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/ModifierHooks.java)。

### 動的なアイテム描画と機械描画

つるはしのitem modelは `loader: tconstruct:tool` を使い、素材別の頭・柄・binding、Modifierの見た目、破損時等のoverrideを組み合わせます。
vanillaのモデルJSONへディレクトリを移すだけでは再現できません。
必要な機能は、ツール個体の部品素材に応じたsprite / tintの合成と、個体状態による外観の切り替えです。
全素材組み合わせを別々のitem IDに展開する方式は、組み合わせ増大と個体状態の扱いを招くため基本方針にはしません。

Smeltery系にもBlock Entity rendererがあり、タンクの液面、Faucet・Channelの流体、鋳造中の液体と生成物などを描きます。
Castingのrendererは進捗に応じて液体やアイテムの不透明度を変えます。したがってTinkers’ Constructも独自描画の移植対象です。
Createの可動構造物を最初から実装する必要はありませんが、静的ブロックモデルだけで完成するModでもありません。

参照: [つるはしモデル](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/resources/assets/tconstruct/models/item/pickaxe.json)、[素材モデル処理](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/client/model/tools/MaterialModel.java)、[鋳造描画](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/client/render/CastingBlockEntityRenderer.java)。

## Horizonとの機能対応

以下は現行WIT・manifest・content登録実装と、上記の原作実装を照合した判定です。

| 原作の機能 | 現行Horizonで可能な範囲 | 不足する接続点 |
| --- | --- | --- |
| レンガ・素材・装飾 | vanillaテンプレートの登録と対応する通常モデル等は候補 | 各ブロックの形状・挙動・モデルが再現可能か個別確認 |
| 素材JSON・ツール性能計算 | Rustの独立したロジックとして先行実装可能 | 独自データloader、registry、reload・同期 |
| ツール個体状態 | Mod内の独立データ構造は作れる | ItemStackごとの状態読み書き、保存・同期、slot間移動 |
| 耐久・修理・採掘・攻撃 | vanillaテンプレートの既存挙動まで | Mod固有のdurability、broken判定、採掘・戦闘hook |
| Part Builder / Tinker Station | 実験UIで表示試作は可能 | サーバー側Block Entity、slot、recipe照合、確定操作、menu同期 |
| Melter / Smeltery | 装飾の一部まで | 機械tick、燃料・温度、液体handler、合金、構造検証 |
| Casting / Faucet / Channel | 固定の見た目の一部まで | 流体転送、cooling進捗、型・出力管理、ワールド内描画 |
| 素材別ツールモデル | 固定モデルの試作まで | 個体状態を参照する部分sprite・tint・Modifier合成 |
| ガイドブック | 限定した文章UIは候補 | Mantle book format、ページ・図・レシピとの連携 |
| ワールド生成・生物・装備 | 対応するvanilla形式のデータは個別候補 | 独自feature / Entity / 装備動作・描画など |

Horizonの `registries` は現在、vanillaのブロック・アイテムをtemplateとして登録する仕組みです。
item定義に任意の独自componentや挙動をJSONで記す口はなく、主要な変更項目はstack sizeとrarityです。
`input` は使用操作の取得に使えますが、それだけで採掘・戦闘・耐久の全hookを実装できるとは扱いません。
`storage` はMod専用保存であり、ItemStackやチャンクに結びつく状態保存を自動で解決しません。

また、`.hmod` に独自JSONを入れられることと、Wasmからそれを読めることは別です。
現行WITに任意パッケージresourceを読むAPIはないため、初期試作は必要なJSONのビルド時埋め込み、将来は権限を限定したresource / reload APIを検討します。
後者を実装するまでは、原作同様のdatapack拡張に対応したと宣言しません。

根拠: [Horizon content定義](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mods/src/content.rs)、[WIT全体](https://github.com/ludovic111/horizon/blob/45bc45b3967f06ab56173fe2b13892004e717e35/crates/horizon-mod-sdk/wit/horizon.wit)。

### データを変換する際の注意

- 原作はMinecraft 1.20.1、調査対象Horizonは26.3です。`recipes` / `loot_tables` / `tags/blocks` 等の旧パスを移すだけではなく、対象版のschemaとIDも変換・検証します。
- `tconstruct:*` のrecipe type、module、predicate、model loaderは実装が必要です。JSONであることは、vanilla互換の証明になりません。
- `forge:*` tagやconditionalを、対象の有効データ・namespace規則へ変換します。単にnamespaceを `c` に置換するだけでは不十分です。
- 鋳造例では液体量90、cooling time 60等の値を持ちます。対象版の単位と消費・生成規則を保存し、別Modや別Minecraft版の慣習値で置き換えません。
- 旧版のツールNBTはHorizon側ItemStack表現へ対応付けます。IDだけ合わせても旧ワールドやツール個体をそのまま読み込めるとは扱いません。

例: [鋳造recipe](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/smeltery/casting/metal/iron/ingot_gold_cast.json)。

## 推奨する段階と受け入れ条件

これは実装の提案です。API追加や本体forkは今回実施していません。

| 段階 | 成果物 | 合格条件 |
| --- | --- | --- |
| 0: 仕様固定 | 対象版・対応データ・初期traitとhookの一覧、原作fixture | 未対応の参照先をすべて把握。原作の入出力を再現できる検証資料がある |
| 1: 計算コア | Rustの素材・部品・stats・trait合成・broken / repairモデル | 同じ入力でJava側と性能・状態・失敗条件が一致する |
| 2: content試作 | 少数の素材・レンガ・通常モデル・通常recipeの `.hmod` | `build` / `check` とゲーム内の登録・表示・通常recipe確認が通る |
| 3: 最初のツール体験 | つるはし、部品作成、組み立て、修理、素材別外観 | 操作はサーバーが検証。採掘・破損・修理・保存・同期が成立する |
| 4: 溶融と鋳造 | 小型Melter → Faucet → Casting Tableの一連動作 | 燃料・液体・型・出力の収支、cooling、途中保存・再読み込みが一致する |
| 5: 溶融炉と拡張 | multiblock Smeltery、合金、Modifier群、他ツール | 構造破壊・再構築、複数流体、chunk境界、負荷、同期を検証する |
| 6: 残りの機能 | 防具、遠距離武器、生物、worldgen、book、他Mod連携 | それぞれの操作・データ形式・描画と互換性を確認する |

段階1と2は現行APIの制約の中で着手できます。段階3以降の成立にはHorizon側の変更が必要です。
最初の遊べる範囲でwoodを使うなら `cultivated`、pickaxeなら `pierce` など、選択した定義のtrait依存も調べます。
仮の無特性素材でAPIだけ検証する場合は、原作互換の完成ツールと区別します。

### Horizon側へ最初に追加すべきAPI

1. Mod管理のItemStack状態と、保存・同期・複製・stack分離・slot移動時の規則。
2. 耐久消費・破損・修理・採掘性能・採掘結果・攻撃等への、対象item限定のサーバーhook。
3. 独自Block Entityの登録・状態・load / unloadと、slot付きmenu・recipe確定処理。
4. 個体状態に基づくツール描画とtooltip。ゲームロジックの結果を表示側へ同期する経路。
5. Melter以降に、fluid型・tank・転送・温度・機械tick・ワールド内流体描画。

Javaの全Forge APIを再現せず、必要な契約を小さく定義します。ホストの型・所有権と権限を明確にし、Wasmの呼び出し回数・メモリ使用量を計測します。
将来上流が予定するAPI番号を、実装済みとしてmanifestに記載することはしません。

## 検証方針

原作には素材manager、ToolStack、stats builder、耐久、採掘、定義・layout・同期packet等のテストがあります。
これらは期待動作の出典として使えますが、Forge / Minecraftのfixtureへ依存するためRustで直接実行できるものではありません。

まず、選んだ素材と部品配列、初期trait、upgrade、damageを入力に、Java側が返すstats・Modifier一覧・broken状態をfixture化します。
Rust側で同じfixtureを検証し、適用順序、境界値、欠損ID、未対応module、serializationの往復を確認します。
必要に応じて原作のJUnit期待値も移します。原作のテストが存在することを「全機能が検証済み」の根拠にはしません。

ゲーム接続後は、次を追加確認します。

- ツールが耐久0で消えずbroken状態になり、修理後に復帰すること。
- 機械のslotやrecipe previewを表示しただけで材料を消費せず、確定時に一度だけ消費すること。
- shift-click、複数人操作、切断、破壊、drop、保存でアイテムが増殖・消失しないこと。
- 素材・Modifierの更新時に、永続データを壊さず必要なstatsだけ再計算すること。
- 鋳造中の保存・chunk unload、出力満杯、燃料切れ、構造破壊でも収支が保たれること。
- 少数動作の確認後、大量のツール状態や機械でtick・描画・Wasmメモリを計測すること。

参照: [原作テスト群](https://github.com/SlimeKnights/TinkersConstruct/tree/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/test/java/slimeknights/tconstruct/library)、[耐久テスト](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/test/java/slimeknights/tconstruct/library/tools/item/ToolItemDurabilityTest.java)。

## この作業場所での配置案

実装を始める場合はルート直下の **`tconstruct/`** をModの実体とし、IDとnamespaceは原作の `tconstruct` を基本とする案です。
内部のRustライブラリとWasmアダプターは同じModディレクトリ内に置き、計算コアをゲームなしでテストできるようにします。
共通ガイドと本調査は `docs/` に残します。
Horizon本体の修正は別checkoutで管理し、対応コミットとpatchをModのREADMEから追えるようにします。

今回は調査段階のため `tconstruct/` や実装用のCargo workspaceは作成していません。

## 実装前に残る確認

- 差分検証に使う公開jarと、ソースコミット・Mantle配布物の正確な対応。
- 初期素材とツールを選んだときに必要になる全Modifier / module / tagの依存閉包。
- Horizonのitem保存形式と採掘・戦闘・menuの内部経路へ、どこでMod hookを挿入するか。
- 使用する各素材・モデル・音源に個別のライセンス例外がないか。
- スキーマ変換後の通常モデル・recipeが、現行Horizonで実際にロードできるか。

これらは未解決の詳細調査項目です。完全移植の工数や完了日を、この静的調査から数値で断定していません。

公式の補助資料: [開発ドキュメント](https://slimeknights.github.io/docs/)、[Tinkers’ Construct 3の遊び方](https://slimeknights.github.io/docs/gameplay/tinkers-construct-3/)。動作判定と仕様固定では上記のコミットにあるソース・データ・テストを優先します。
