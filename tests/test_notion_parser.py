from __future__ import annotations

import sys
import unittest
from datetime import date
from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

from work_log.notion_parser import parse_outline_text


class NotionParserTest(unittest.TestCase):
    def test_normalizes_standalone_impact_and_ignores_today(self) -> None:
        outline = """
## today
- Review PR

## done
- Settings screen fix
- backlog auto task validation

## impact
- UX improvement
- Reduced production team workload

## improvement
- Investigated retry button design
"""
        log = parse_outline_text(outline, date(2026, 3, 9), source_page="page-1")

        self.assertEqual(2, len(log.done))
        self.assertEqual("Settings screen fix", log.done[0].task)
        self.assertEqual("UX improvement", log.done[0].impact)
        self.assertEqual("Reduced production team workload", log.done[1].impact)
        self.assertEqual(["Investigated retry button design"], log.improvements)

    def test_allows_missing_impact(self) -> None:
        outline = """
## done
- Investigated timezone handling

## notes
- Follow up with QA tomorrow
"""
        log = parse_outline_text(outline, date(2026, 3, 10))

        self.assertEqual(1, len(log.done))
        self.assertIsNone(log.done[0].impact)
        self.assertEqual(["Follow up with QA tomorrow"], log.notes)

    def test_attaches_nested_impact_to_done_item(self) -> None:
        outline = """
## done
- backlog import validation
  impact: Saved manual registration time
"""
        log = parse_outline_text(outline, date(2026, 3, 11))

        self.assertEqual("Saved manual registration time", log.done[0].impact)


if __name__ == "__main__":
    unittest.main()
