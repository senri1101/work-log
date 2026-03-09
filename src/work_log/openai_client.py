from __future__ import annotations

import json
from urllib import error, request

OPENAI_API_BASE = "https://api.openai.com/v1"


class OpenAIAPIError(RuntimeError):
    """Raised when an OpenAI API request fails."""


class OpenAIClient:
    def __init__(self, api_key: str, model: str) -> None:
        self._api_key = api_key
        self._model = model

    def generate_markdown(self, system_prompt: str, user_prompt: str) -> str:
        payload = {
            "model": self._model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
        }
        data = json.dumps(payload).encode("utf-8")
        req = request.Request(
            f"{OPENAI_API_BASE}/chat/completions",
            data=data,
            headers={
                "Authorization": f"Bearer {self._api_key}",
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with request.urlopen(req) as response:
                body = json.load(response)
        except error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace")
            raise OpenAIAPIError(
                f"OpenAI API request failed ({exc.code} {exc.reason}): {detail}"
            ) from exc
        except error.URLError as exc:
            raise OpenAIAPIError(f"Failed to reach OpenAI API: {exc.reason}") from exc

        choices = body.get("choices", [])
        if not choices:
            raise OpenAIAPIError("OpenAI API returned no choices.")
        content = choices[0].get("message", {}).get("content", "")
        if not content:
            raise OpenAIAPIError("OpenAI API returned an empty message.")
        return content.strip() + "\n"
