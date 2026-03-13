# work-log

`work-log` は、Tauri 製の macOS 日報アプリです。  
その日の作業を Markdown と Git に残しやすい形に整えます。

アプリ本体の repo と、日報の実データ repo は分離して使えます。  
初回起動後に保存先として任意のログ用 repo を設定すると、その配下に `daily/`, `weekly/`, `reviews/`, `achievements/`, `tech-notes/`, `.work-log-state/` を作成します。

## いまの構成

```text
work-log/
├── web/                  # Vite + React + BlockNote フロントエンド
├── src-tauri/            # Tauri の Rust バックエンド
├── docs/                 # LP / GitHub Pages
├── templates/            # 生成テンプレート
└── src/work_log/         # 任意: Gemini 生成用の Python CLI
```

## できること

- 1 枚の Markdown 日報をそのまま編集
- `#`, `##`, `###`, `- [ ]`, `- [x]`, `- [/]`, `- ` を書くと、その場で見やすく整形表示
- チェックボックスはクリックでも切り替え可能
- その日に書いた内容全体を `daily/*.md` に保存
- 未完了タスクを翌日に持ち越し
- `Cmd/Ctrl + -`, `Cmd/Ctrl + =`, `Cmd/Ctrl + 0` で UI 全体を拡大縮小
- 任意のログ用 repo に保存
- UI から commit / push
- Gemini で実績ログや自己評価ドラフトを生成

保存される Markdown 例:

```md
# 2026-03-12

## ✅ 今日やること

### 🚨 今日必達
- [ ] 週次ミーティングの準備
  - [x] アジェンダ整理
  - [ ] 共有メモ更新
  - 関連リンクをまとめておく

### 🐻 必達以外
- [/] ダッシュボード改善

## 📝 メモ / 気づき
- support: 依頼内容の確認
- learning: markdown workflow

## 🐕 保留
- [ ] 通知文言の見直し
  - 来週まとめて調整する
```

## データ保存先

- 編集中の状態: `<ログ用repo>/.work-log-state/entries/YYYY/YYYY-MM-DD.json`
- 正式な日報兼バックアップ: `<ログ用repo>/daily/YYYY/YYYY-MM-DD.md`

`.work-log-state` は Git 管理しません。  
Git に残すのは `daily/` 以下の Markdown が中心です。  
JSON が無くても `daily/*.md` から復元できます。

## セットアップ

### 1. Tauri アプリを使う

```bash
git clone https://github.com/senri1101/work-log.git
cd work-log
pnpm install
pnpm tauri dev
```

### 1.5 配布用ビルドを作る

```bash
pnpm tauri build --bundles app
```

生成物の例:

- `src-tauri/target/release/bundle/macos/work-log.app`

`.dmg` まで欲しい場合は `pnpm tauri build` を使えますが、環境によっては `bundle_dmg.sh` 側で失敗することがあります。  
まずは `.app` 配布を基準にするのが安全です。

未署名ビルドなので、他の Mac へ配る場合は署名や notarization を別途検討してください。

### 2. Gemini 生成を使う

```bash
cp .env.example .env
uv sync
uv run python -m unittest discover -s tests -v
```

## 日報アプリの使い方

1. 初回は左側で `選ぶ` からログ用 repo のパスを設定する
2. 1 枚の画面でブロックとして日報を書く
3. チェックリストはクリックでも状態変更できる
4. `Enter`, `Tab`, `Shift + Tab` で行とインデントを編集する
5. 保存すると `<ログ用repo>/daily/YYYY/YYYY-MM-DD.md` が更新される
6. `公開` を押すと、UI から commit と GitHub push まで実行できる
7. 左側で `保存時に commit` / `保存時に push` を設定すると自動化できる

## Gemini 生成コマンド

- `uv run work-log generate-achievement --from YYYY-MM-DD --to YYYY-MM-DD --slug NAME`
- `uv run work-log generate-review --period YYYY-H1 [--from YYYY-MM-DD --to YYYY-MM-DD]`

## 環境変数

- `GEMINI_API_KEY`
- `GEMINI_MODEL`
- `TZ`

`GEMINI_MODEL` の既定値は `gemini-2.5-flash-lite` です。

## GitHub 設定

ログ用 repo を作る場合:

```bash
gh auth login -h github.com -p https -w
gh repo create YOUR_NAME/work-log-data --private --clone --add-readme
```

## LP / GitHub Pages

LP は `docs/` 配下の静的ファイルとして管理しています。  
`main` に push されると `.github/workflows/deploy-pages.yml` で GitHub Pages に反映されます。

## Third-Party Licenses

利用ライブラリのライセンス情報は [THIRD_PARTY_LICENSES.md](/Users/senri.gotoda/Desktop/Repositories/work-log/THIRD_PARTY_LICENSES.md) を参照してください。

## メモ

- Tauri アプリは `daily/*.md` をフルログとして保存します。UI 上の見た目が整っていても、実体は Markdown です。
- 新しい日付を開くと、前日の `[ ]` と `[/]` が階層ごと引き継がれます。`[x]` は引き継ぎません。
- `.work-log-state` が無い場合でも `daily/*.md` から復元できます。
- 旧 `done/support/improvements/learning/notes` 形式の Markdown や旧 JSON は読み込み互換を残しています。
- `保存時に push` を ON にすると、保存時に commit も自動で有効になります。
- Python CLI は既存の日報 Markdown を読み込み、Gemini で成果や自己評価を生成します。
- `uv.lock` をコミットしているため、Gemini 生成の Python 環境は固定できます。
