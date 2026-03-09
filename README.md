# work-log

`work-log` is a private personal knowledge base for syncing daily Notion logs into a Git repository and generating achievement or review documents from those logs.

## Repository layout

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

## Daily log model

Notion remains the authoring surface. The canonical GitHub format stores only durable sections and drops `today`.

```md
# 2026-03-09

## done
- task: Settings screen fix
  impact: UX improvement
- task: backlog auto task validation
  impact: Reduced production team workload

## support
- Responded to Wang's request

## improvements
- Investigated retry button design

## learning
- timezone handling

## notes
- Researched release visibility controls
```

## Setup

1. Copy `.env.example` to `.env`.
2. Fill in the Notion and OpenAI credentials.
3. Run commands with `PYTHONPATH=src`.

```bash
cp .env.example .env
PYTHONPATH=src python -m unittest discover -s tests -v
PYTHONPATH=src python -m work_log sync-daily --date 2026-03-09
PYTHONPATH=src python -m work_log generate-review --period 2026-H1 --from 2026-01-01 --to 2026-06-30
```

## CLI

- `sync-daily --date YYYY-MM-DD [--dry-run]`
- `sync-range --from YYYY-MM-DD --to YYYY-MM-DD [--dry-run]`
- `generate-achievement --from YYYY-MM-DD --to YYYY-MM-DD --slug NAME [--dry-run]`
- `generate-review --period PERIOD [--from YYYY-MM-DD --to YYYY-MM-DD] [--dry-run]`

## Environment variables

- `NOTION_TOKEN`
- `NOTION_DATABASE_ID`
- `NOTION_DATE_PROPERTY`
- `OPENAI_API_KEY`
- `OPENAI_MODEL`
- `TZ`

## GitHub setup

The current local `gh` token is invalid. Re-authenticate before creating the private repository:

```bash
gh auth login -h github.com -p https -w
gh repo create senri1101/work-log --private --source=. --remote=origin --push
```

## Notes

- Notion heading aliases `task`, `tasks`, `todo`, `improvement`, and `improvements` are normalized automatically.
- Standalone `impact` sections are accepted on import and normalized into `done` item impacts on save.
- `learning` is optional. Empty optional sections are omitted from the saved markdown.
