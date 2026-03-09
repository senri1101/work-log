# work-log

`work-log` は、Notion の日報を Git 管理の Markdown に同期し、実績ログや自己評価ドラフトを生成するための個人用リポジトリです。

## リポジトリ構成

```text
work-log/
├── achievements/
├── daily/
├── reviews/
├── tech-notes/
├── templates/
├── tests/
├── weekly/
└── src/work_log/
```

## 日報の保存形式

Notion を入力元にしつつ、GitHub 側には長期保存したいログだけを残します。`today` は保存せず、`done` に対する `impact` を主軸に正規化します。

```md
# 2026-03-09

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

## セットアップ

1. `.env.example` を `.env` にコピーします。
2. Notion と OpenAI の認証情報を設定します。
3. `uv` で環境を同期します。
4. 以降のコマンドは `uv run` で実行します。

```bash
cp .env.example .env
uv sync
uv run python -m unittest discover -s tests -v
uv run work-log sync-daily --date 2026-03-09
uv run work-log generate-review --period 2026-H1 --from 2026-01-01 --to 2026-06-30
```

## CLI

- `sync-daily --date YYYY-MM-DD [--dry-run]`
- `sync-range --from YYYY-MM-DD --to YYYY-MM-DD [--dry-run]`
- `generate-achievement --from YYYY-MM-DD --to YYYY-MM-DD --slug NAME [--dry-run]`
- `generate-review --period PERIOD [--from YYYY-MM-DD --to YYYY-MM-DD] [--dry-run]`

## 環境変数

- `NOTION_TOKEN`
- `NOTION_DATABASE_ID`
- `NOTION_DATE_PROPERTY`
- `OPENAI_API_KEY`
- `OPENAI_MODEL`
- `TZ`

## Notion 側の書き方

見出し名は機械処理の都合で英語固定です。本文は日本語で問題ありません。

```md
## today
- PRレビュー
- API確認

## done
- 設定画面修正
  impact: UX改善
- backlog自動タスク検証
  impact: 制作チームの作業削減

## support
- 王さんの確認対応

## improvement
- retryボタン検討

## learning
- timezone handling

## notes
- 日々の言葉の公開制御調査
```

## GitHub 設定

このマシンの `gh` トークンは現在無効です。private repository を作成する前に再認証してください。

```bash
gh auth login -h github.com -p https -w
gh repo create senri1101/work-log --private --source=. --remote=origin --push
```

## GitHub Actions の Secrets

- `NOTION_TOKEN`
- `NOTION_DATABASE_ID`
- `NOTION_DATE_PROPERTY`
- `OPENAI_API_KEY`
- `OPENAI_MODEL`

## 補足

- `task`, `tasks`, `todo`, `improvement`, `improvements` の見出し揺れは自動で正規化されます。
- 独立した `impact` セクションがあっても、保存時には各 `done` 項目に紐づけます。
- `learning` は任意項目です。空の場合は Markdown に出力しません。
- `uv.lock` をコミットしているため、CI とローカルで同じ環境を使えます。
