#!/usr/bin/env python3
"""list_tds.py — 列 ada-mock TDS 状态, 给出未锁定数量 / 缺口汇总.

用法:
    python scripts/list_tds.py            # 打印所有 TDS 摘要
    python scripts/list_tds.py --json     # JSON 输出
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TDS_DIR = ROOT / "docs" / "tds"

# 状态行形如 "> 状态: 锁定" (Markdown blockquote) 或 "**状态**: 锁定"
STATUS_RE = re.compile(r"^\s*(?:>\s*)?(?:\*\*)?状态(?:\*\*)?:\s*(?P<status>.+?)\s*$", re.MULTILINE)
ID_RE = re.compile(r"^#\s+TDS-(?P<id>[\w\-]+)", re.MULTILINE)


def parse_one(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    m_id = ID_RE.search(text)
    m_status = STATUS_RE.search(text)
    return {
        "id": m_id.group("id") if m_id else path.stem,
        "status": (m_status.group("status").strip() if m_status else "未知"),
        "path": str(path.relative_to(ROOT)),
    }


def main() -> int:
    if not TDS_DIR.exists():
        print(f"TDS 目录不存在: {TDS_DIR}", file=sys.stderr)
        return 1
    files = sorted(TDS_DIR.glob("TDS-*.md"))
    items = [parse_one(p) for p in files]
    locked = [i for i in items if i["status"] == "锁定"]
    drafts = [i for i in items if i["status"] not in ("锁定", "废止")]

    if "--json" in sys.argv:
        print(json.dumps({
            "total": len(items),
            "locked": len(locked),
            "draft": len(drafts),
            "items": items,
        }, ensure_ascii=False, indent=2))
    else:
        print(f"== TDS 状态汇总 ==  total={len(items)}  locked={len(locked)}  draft={len(drafts)}")
        for i in items:
            mark = "[OK]" if i["status"] == "锁定" else "[DRF]" if i["status"] == "草案" else "[---]"
            print(f"  {mark}  TDS-{i['id']:<20}  {i['status']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
