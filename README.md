# work-log

`work-log` は、Tauri 製の macOS 日報アプリで日々の作業を記録し、その内容を Git 管理しやすい Markdown に保存するための個人用リポジトリです。  
必要に応じて、蓄積した日報から Gemini で実績ログや自己評価ドラフトも生成できます。

## いまの構成

```text
work-log/
├── desktop/              # Tauri のフロントエンド
├── src-tauri/            # Tauri の Rust バックエンド
├── daily/                # 生成される日報 Markdown
├── achievements/         # 実績ログ
├── reviews/              # 自己評価ドラフト
├── templates/
└── src/work_log/         # Gemini 生成用の Python CLI
```

## アプリの役割

アプリでは `today` をチェックボックス付きで管理します。  
終わった項目にチェックを入れて `impact` を書くと、保存時にチェック済みの項目だけが `done` として Markdown に出力されます。

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

- 編集中の状態: `.work-log-state/entries/YYYY/YYYY-MM-DD.json`
- 正式な日報: `daily/YYYY/YYYY-MM-DD.md`

`.work-log-state` は Git 管理しません。  
Git に残すのは `daily/` 以下の Markdown が中心です。

## セットアップ

### 1. Tauri アプリを使う

```bash
cd /Users/senri.gotoda/Desktop/Repositories/work-log
pnpm install
pnpm tauri dev
```

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
5. 保存すると `daily/YYYY/YYYY-MM-DD.md` が更新される

## Gemini 生成コマンド

- `uv run work-log generate-achievement --from YYYY-MM-DD --to YYYY-MM-DD --slug NAME`
- `uv run work-log generate-review --period YYYY-H1 [--from YYYY-MM-DD --to YYYY-MM-DD]`

## 環境変数

- `GEMINI_API_KEY`
- `GEMINI_MODEL`
- `TZ`

`GEMINI_MODEL` の既定値は `gemini-2.5-flash-lite` です。

## GitHub 設定

private repository を作成する場合:

```bash
gh auth login -h github.com -p https -w
gh repo create senri1101/work-log --private --source=. --remote=origin --push
```

## メモ

- Tauri アプリは保存時に `today` をそのまま Markdown へ出さず、チェック済み項目だけを `done` に変換します。
- Python CLI は既存の日報 Markdown を読み込み、Gemini で成果や自己評価を生成します。
- `uv.lock` をコミットしているため、Gemini 生成の Python 環境は固定できます。
