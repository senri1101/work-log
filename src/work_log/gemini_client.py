from __future__ import annotations

import json
from urllib import error, parse, request

GEMINI_API_BASE = "https://generativelanguage.googleapis.com/v1beta"


class GeminiAPIError(RuntimeError):
    """Raised when a Gemini API request fails."""


class GeminiClient:
    def __init__(self, api_key: str, model: str) -> None:
        self._api_key = api_key
        self._model = model

    def generate_markdown(self, system_prompt: str, user_prompt: str) -> str:
        payload = {
            "system_instruction": {
                "parts": [{"text": system_prompt}],
            },
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": user_prompt}],
                }
            ],
            "generationConfig": {
                "temperature": 0.2,
            },
        }
        data = json.dumps(payload).encode("utf-8")
        model_path = parse.quote(self._model, safe="")
        req = request.Request(
            f"{GEMINI_API_BASE}/models/{model_path}:generateContent",
            data=data,
            headers={
                "x-goog-api-key": self._api_key,
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with request.urlopen(req) as response:
                body = json.load(response)
        except error.HTTPError as exc:
            detail = exc.read().decode("utf-8", "replace")
            raise GeminiAPIError(
                f"Gemini API request failed ({exc.code} {exc.reason}): {detail}"
            ) from exc
        except error.URLError as exc:
            raise GeminiAPIError(f"Failed to reach Gemini API: {exc.reason}") from exc

        prompt_feedback = body.get("promptFeedback", {})
        if prompt_feedback.get("blockReason"):
            raise GeminiAPIError(
                f"Gemini prompt was blocked: {prompt_feedback['blockReason']}"
            )

        candidates = body.get("candidates", [])
        if not candidates:
            raise GeminiAPIError("Gemini API returned no candidates.")

        parts = candidates[0].get("content", {}).get("parts", [])
        texts = [part.get("text", "") for part in parts if part.get("text")]
        if not texts:
            finish_reason = candidates[0].get("finishReason")
            raise GeminiAPIError(
                f"Gemini API returned no text content. finishReason={finish_reason}"
            )
        return "".join(texts).strip() + "\n"
