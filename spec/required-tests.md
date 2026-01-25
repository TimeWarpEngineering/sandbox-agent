# Required Tests

- Session manager streams JSONL line-by-line for Claude/Codex/Amp and yields incremental events.
- `/sessions/{id}/messages` returns immediately while background ingestion populates `/events` and `/events/sse`.
- SSE subscription delivers live events after the initial offset batch.
- OpenCode server mode: create session, send prompt, and receive SSE events filtered to the session.
- OpenCode question/permission reply endpoints forward to server APIs.
