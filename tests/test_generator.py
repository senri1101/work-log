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
from work_log.generator import build_review_prompts
from work_log.models import DailyLog, DoneItem


class GeneratorTest(unittest.TestCase):
    def test_review_prompt_prioritizes_impact(self) -> None:
        logs = [
            DailyLog(
                entry_date=date(2026, 3, 9),
                done=[DoneItem(task="Settings screen fix", impact="UX improvement")],
                support=["Helped QA verify edge cases"],
                improvements=["Investigated retry design"],
            )
        ]
        system_prompt, user_prompt = build_review_prompts(
            logs,
            "2026-H1",
            date(2026, 1, 1),
            date(2026, 6, 30),
        )

        self.assertIn("impact の反復テーマを最優先", system_prompt)
        self.assertIn("impact: UX improvement", user_prompt)
        self.assertIn("support:", user_prompt)

    def test_generate_review_writes_markdown_file(self) -> None:
        with tempfile.TemporaryDirectory() as tempdir:
            root = Path(tempdir)
            daily_dir = root / "daily/2026"
            daily_dir.mkdir(parents=True)
            (daily_dir / "2026-03-09.md").write_text(
                """# 2026-03-09

## done
- task: Settings screen fix
  impact: UX improvement
""",
                encoding="utf-8",
            )
            env = {
                "GEMINI_API_KEY": "token",
                "GEMINI_MODEL": "test-model",
            }
            with patch.dict(os.environ, env, clear=False):
                with patch("work_log.cli.GeminiClient") as client_cls:
                    client_cls.return_value.generate_markdown.return_value = (
                        "# 2026-H1\n\n## summary\n\nShipped the work.\n"
                    )
                    exit_code = cli.main(
                        [
                            "generate-review",
                            "--period",
                            "2026-H1",
                            "--from",
                            "2026-03-09",
                            "--to",
                            "2026-03-09",
                        ],
                        root=root,
                    )

            self.assertEqual(0, exit_code)
            output = (root / "reviews/2026-H1.md").read_text(encoding="utf-8")
            self.assertIn("# 2026-H1", output)


if __name__ == "__main__":
    unittest.main()
