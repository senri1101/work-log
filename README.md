# work-log

`work-log` は、Tauri 製の macOS 日報アプリです。  
その日の作業を Markdown と Git に残しやすい形に整えます。

アプリ本体の repo と、日報の実データ repo は分離して使えます。  
初回起動後に保存先として任意のログ用 repo を設定すると、その配下に `daily/`, `weekly/`, `reviews/`, `achievements/`, `tech-notes/`, `.work-log-state/` を作成します。

## いまの構成

```text
work-log/
├── desktop/              # Tauri のフロントエンド
├── src-tauri/            # Tauri の Rust バックエンド
├── templates/
└── src/work_log/         # Gemini 生成用の Python CLI
```

## できること

- `today` をチェック付きで管理
- 完了したものだけを `done` として Markdown 化
- 未完了タスクを翌日に持ち越し
- 任意のログ用 repo に保存
- UI から commit / push
- Gemini で実績ログや自己評価ドラフトを生成

保存される Markdown 例:

```md
# 2026-03-10

## done
- task: 設定画面修正
  impact: UX改善
- task: backlog自動タスク検証
  impact: 制作チームの作業削減

## support
- 王さんの確認対応

## improvements
- retryボタン検討

## learning
- timezone handling

## notes
- 日々の言葉の公開制御調査
```

## データ保存先

- 編集中の状態: `<ログ用repo>/.work-log-state/entries/YYYY/YYYY-MM-DD.json`
- 正式な日報: `<ログ用repo>/daily/YYYY/YYYY-MM-DD.md`

`.work-log-state` は Git 管理しません。  
Git に残すのは `daily/` 以下の Markdown が中心です。

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

1. `today` に今日やることを追加する
2. 完了したらチェックを入れる
3. 必要なら `impact` を書く
4. `support`, `improvements`, `learning`, `notes` を追記する
5. 初回は左側で `選ぶ` からログ用 repo のパスを設定する
6. 保存すると `<ログ用repo>/daily/YYYY/YYYY-MM-DD.md` が更新される
7. `保存して push` を押すと、UI から commit と GitHub push まで実行できる
8. 左側で `保存時に commit` / `保存時に push` を設定すると自動化できる

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

## メモ

- Tauri アプリは保存時に `today` をそのまま Markdown へ出さず、チェック済み項目だけを `done` に変換します。
- 新しい日付を開くと、前日の未完了タスクだけを `today` に引き継ぎます。
- `保存時に push` を ON にすると、保存時に commit も自動で有効になります。
- Python CLI は既存の日報 Markdown を読み込み、Gemini で成果や自己評価を生成します。
- `uv.lock` をコミットしているため、Gemini 生成の Python 環境は固定できます。
