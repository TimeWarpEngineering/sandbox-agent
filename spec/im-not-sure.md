# Open Questions / Ambiguities

- OpenCode server HTTP paths and payloads may differ; current implementation assumes `POST /session`, `POST /session/{id}/prompt`, and `GET /event/subscribe` with JSON `data:` SSE frames.
- OpenCode question/permission reply endpoints are assumed as `POST /question/reply`, `/question/reject`, `/permission/reply` with `requestID` fields; confirm actual API shape.
- SSE events may not always include `sessionID`/`sessionId` fields; confirm if filtering should use a different field.
