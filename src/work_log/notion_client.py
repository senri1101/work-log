from __future__ import annotations

import json
from datetime import date
from typing import Any
from urllib import error, request

from work_log.models import NotionPage

NOTION_API_BASE = "https://api.notion.com/v1"
NOTION_VERSION = "2022-06-28"


class NotionAPIError(RuntimeError):
    """Raised when a Notion API request fails."""


class NotionClient:
    def __init__(self, token: str, database_id: str, date_property: str) -> None:
        self._token = token
        self._database_id = database_id
        self._date_property = date_property

    def query_pages(self, start_date: date, end_date: date) -> list[NotionPage]:
        payload = {
            "filter": {
                "and": [
                    {
                        "property": self._date_property,
                        "date": {"on_or_after": start_date.isoformat()},
                    },
                    {
                        "property": self._date_property,
                        "date": {"on_or_before": end_date.isoformat()},
                    },
                ]
            },
            "sorts": [{"property": self._date_property, "direction": "ascending"}],
        }
        results: list[dict[str, Any]] = []
        next_cursor: str | None = None

        while True:
            body = dict(payload)
            if next_cursor:
                body["start_cursor"] = next_cursor
            response = self._request(
                "POST",
                f"/databases/{self._database_id}/query",
                body,
            )
            results.extend(response.get("results", []))
            if not response.get("has_more"):
                break
            next_cursor = response.get("next_cursor")

        pages: list[NotionPage] = []
        for page in results:
            entry_date = extract_page_date(page, self._date_property)
            if not entry_date:
                continue
            pages.append(
                NotionPage(
                    page_id=page["id"],
                    entry_date=entry_date,
                    title=extract_page_title(page),
                    outline_text=self.render_page_outline(page["id"]),
                )
            )
        return pages

    def render_page_outline(self, page_id: str) -> str:
        blocks = self._fetch_children(page_id)
        lines = render_blocks(blocks)
        return "\n".join(lines)

    def _fetch_children(self, block_id: str) -> list[dict[str, Any]]:
        results: list[dict[str, Any]] = []
        next_cursor: str | None = None

        while True:
            path = f"/blocks/{block_id}/children?page_size=100"
            if next_cursor:
                path += f"&start_cursor={next_cursor}"
            response = self._request("GET", path)
            for block in response.get("results", []):
                if block.get("has_children"):
                    block["_children"] = self._fetch_children(block["id"])
                results.append(block)
            if not response.get("has_more"):
                break
            next_cursor = response.get("next_cursor")
        return results

    def _request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        headers = {
            "Authorization": f"Bearer {self._token}",
            "Notion-Version": NOTION_VERSION,
        }
        data: bytes | None = None
        if payload is not None:
            headers["Content-Type"] = "application/json"
            data = json.dumps(payload).encode("utf-8")
        req = request.Request(
            f"{NOTION_API_BASE}{path}",
            data=data,
            headers=headers,
            method=method,
        )
        try:
            with request.urlopen(req) as response:
                return json.load(response)
        except error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace")
            raise NotionAPIError(
                f"Notion API request failed ({exc.code} {exc.reason}): {detail}"
            ) from exc
        except error.URLError as exc:
            raise NotionAPIError(f"Failed to reach Notion API: {exc.reason}") from exc


def extract_page_date(page: dict[str, Any], property_name: str) -> date | None:
    prop = page.get("properties", {}).get(property_name)
    if not prop:
        return None
    date_value = prop.get("date")
    if not date_value or not date_value.get("start"):
        return None
    return date.fromisoformat(date_value["start"][:10])


def extract_page_title(page: dict[str, Any]) -> str | None:
    for prop in page.get("properties", {}).values():
        if prop.get("type") != "title":
            continue
        title = rich_text_to_plain(prop.get("title", []))
        if title:
            return title
    return None


def render_blocks(blocks: list[dict[str, Any]], indent: int = 0) -> list[str]:
    lines: list[str] = []
    for block in blocks:
        line = render_block_line(block, indent)
        if line:
            lines.append(line)

        children = block.get("_children", [])
        if children:
            child_indent = indent
            if block["type"] in {"bulleted_list_item", "numbered_list_item", "to_do"}:
                child_indent += 1
            lines.extend(render_blocks(children, child_indent))
    return lines


def render_block_line(block: dict[str, Any], indent: int) -> str | None:
    block_type = block["type"]
    payload = block.get(block_type, {})
    text = rich_text_to_plain(payload.get("rich_text", []))

    if block_type == "heading_1":
        return f"# {text}" if text else None
    if block_type == "heading_2":
        return f"## {text}" if text else None
    if block_type == "heading_3":
        return f"### {text}" if text else None
    if block_type in {"bulleted_list_item", "numbered_list_item", "to_do"}:
        prefix = "  " * indent
        return f"{prefix}- {text}" if text else None
    if block_type in {"paragraph", "callout", "quote", "toggle"}:
        prefix = "  " * indent
        return f"{prefix}{text}" if text else None
    return None


def rich_text_to_plain(items: list[dict[str, Any]]) -> str:
    return "".join(item.get("plain_text", "") for item in items).strip()
