from __future__ import annotations

import os
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path
from unittest.mock import patch

SRC = Path(__file__).resolve().parents[1] / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

from work_log import cli
from work_log.models import NotionPage


class CLITest(unittest.TestCase):
    def test_sync_daily_writes_canonical_markdown_without_today(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            page = NotionPage(
                page_id="page-1",
                entry_date=date(2026, 3, 9),
                title="2026-03-09",
                outline_text="""
## today
- Review PR

## done
- Settings screen fix
  impact: UX improvement

## support
- Helped Wang with verification
""",
            )
            env = {
                "NOTION_TOKEN": "token",
                "NOTION_DATABASE_ID": "db",
                "NOTION_DATE_PROPERTY": "Date",
            }
            with patch.dict(os.environ, env, clear=False):
                with patch("work_log.cli.NotionClient") as client_cls:
                    client_cls.return_value.query_pages.return_value = [page]
                    exit_code = cli.main(
                        ["sync-daily", "--date", "2026-03-09"],
                        root=root,
                    )

            self.assertEqual(0, exit_code)
            output = (root / "daily/2026/2026-03-09.md").read_text(encoding="utf-8")
            self.assertNotIn("## today", output)
            self.assertIn("impact: UX improvement", output)
            self.assertIn("## support", output)

    def test_sync_range_dry_run_does_not_write_files(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            page = NotionPage(
                page_id="page-1",
                entry_date=date(2026, 3, 9),
                title="2026-03-09",
                outline_text="""
## done
- Settings screen fix
""",
            )
            env = {
                "NOTION_TOKEN": "token",
                "NOTION_DATABASE_ID": "db",
                "NOTION_DATE_PROPERTY": "Date",
            }
            with patch.dict(os.environ, env, clear=False):
                with patch("work_log.cli.NotionClient") as client_cls:
                    client_cls.return_value.query_pages.return_value = [page]
                    exit_code = cli.main(
                        [
                            "sync-range",
                            "--from",
                            "2026-03-09",
                            "--to",
                            "2026-03-09",
                            "--dry-run",
                        ],
                        root=root,
                    )

            self.assertEqual(0, exit_code)
            self.assertFalse((root / "daily/2026/2026-03-09.md").exists())


if __name__ == "__main__":
    unittest.main()
