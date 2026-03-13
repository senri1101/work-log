from __future__ import annotations

import sys
import unittest
from datetime import date
from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

from work_log.models import TaskStatus
from work_log.notion_parser import parse_outline_text


class NotionParserTest(unittest.TestCase):
    def test_parses_new_canonical_markdown(self) -> None:
        outline = """
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

## 🐕 保留
- [ ] 通知文言の見直し
  - 来週まとめて調整する
"""
        log = parse_outline_text(outline, date(2026, 3, 12))

        self.assertEqual(1, len(log.must_do_tasks))
        self.assertEqual("週次ミーティングの準備", log.must_do_tasks[0].text)
        self.assertEqual(TaskStatus.TODO, log.must_do_tasks[0].status)
        self.assertEqual(2, len(log.must_do_tasks[0].children))
        self.assertEqual(TaskStatus.DONE, log.must_do_tasks[0].children[0].status)
        self.assertEqual(["関連リンクをまとめておく"], log.must_do_tasks[0].notes)
        self.assertEqual(TaskStatus.DOING, log.queued_tasks[0].status)
        self.assertEqual(["support: 依頼内容の確認"], log.memo_lines)
        self.assertEqual(["来週まとめて調整する"], log.pending_tasks[0].notes)

    def test_reads_legacy_done_and_memo_sections(self) -> None:
        outline = """
## today
- Review PR

## done
- Settings screen fix
  impact: UX improvement

## support
- Helped Wang with verification

## improvement
- Investigated retry button design
"""
        log = parse_outline_text(outline, date(2026, 3, 9), source_page="page-1")

        self.assertEqual(["page-1"], log.source_pages)
        self.assertEqual(2, len(log.queued_tasks))
        self.assertEqual(TaskStatus.TODO, log.queued_tasks[0].status)
        self.assertEqual("Review PR", log.queued_tasks[0].text)
        self.assertEqual(TaskStatus.DONE, log.queued_tasks[1].status)
        self.assertEqual(["impact: UX improvement"], log.queued_tasks[1].notes)
        self.assertEqual(
            [
                "support: Helped Wang with verification",
                "improvement: Investigated retry button design",
            ],
            log.memo_lines,
        )


if __name__ == "__main__":
    unittest.main()
