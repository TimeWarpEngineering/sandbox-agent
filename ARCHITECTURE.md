# Architecture

This document covers three key architectural areas of the sandbox-daemon system.

## Agent Schema Pipeline

The schema pipeline extracts type definitions from AI coding agents and converts them to a universal format.

### Schema Extraction

TypeScript extractors in `resources/agent-schemas/src/` pull schemas from each agent:

| Agent | Source | Extractor |
|-------|--------|-----------|
| Claude | `claude --output-format json --json-schema` | `claude.ts` |
| Codex | `codex app-server generate-json-schema` | `codex.ts` |
| OpenCode | GitHub OpenAPI spec | `opencode.ts` |
| Amp | Scrapes ampcode.com docs | `amp.ts` |

All extractors include fallback schemas for when CLIs or URLs are unavailable.

**Output:** JSON schemas written to `resources/agent-schemas/artifacts/json-schema/`

### Rust Type Generation

The `server/packages/extracted-agent-schemas/` package generates Rust types at build time:

- `build.rs` reads JSON schemas and uses the `typify` crate to generate Rust structs
- Generated code is written to `$OUT_DIR/{agent}.rs`
- Types are exposed via `include!()` macros in `src/lib.rs`

```
resources/agent-schemas/artifacts/json-schema/*.json
        ↓ (build.rs + typify)
$OUT_DIR/{claude,codex,opencode,amp}.rs
        ↓ (include!)
extracted_agent_schemas::{claude,codex,opencode,amp}::*
```

### Universal Schema

The `server/packages/universal-agent-schema/` package defines agent-agnostic types:

**Core types** (`src/lib.rs`):
- `UniversalEvent` - Wrapper with id, timestamp, session_id, agent, data
- `UniversalEventData` - Enum: Message, Started, Error, QuestionAsked, PermissionAsked, Unknown
- `UniversalMessage` - Parsed (role, parts, metadata) or Unparsed (raw JSON)
- `UniversalMessagePart` - Text, ToolCall, ToolResult, FunctionCall, FunctionResult, File, Image, Error, Unknown

**Converters** (`src/agents/{claude,codex,opencode,amp}.rs`):
- Each agent has a converter module that transforms native events to universal format
- Conversions are best-effort; unparseable data preserved in `Unparsed` or `Unknown` variants

## Session Management

Sessions track agent conversations with in-memory state.

### Storage

Sessions are stored in an in-memory `HashMap<String, SessionState>` inside `SessionManager`:

```rust
struct SessionManager {
    sessions: Mutex<HashMap<String, SessionState>>,
    // ...
}
```

There is no disk persistence. Sessions are ephemeral and lost on server restart.

### SessionState

Each session tracks:

| Field | Purpose |
|-------|---------|
| `session_id` | Client-provided identifier |
| `agent` | Agent type (Claude, Codex, OpenCode, Amp) |
| `agent_mode` | Operating mode (build, plan, custom) |
| `permission_mode` | Permission handling (default, plan, bypass) |
| `model` | Optional model override |
| `events: Vec<UniversalEvent>` | Full event history |
| `pending_questions` | Question IDs awaiting reply |
| `pending_permissions` | Permission IDs awaiting reply |
| `broadcaster` | Tokio broadcast channel for SSE streaming |
| `ended` | Whether agent process has terminated |

### Lifecycle

```
POST /v1/sessions/{sessionId}     Create session, auto-install agent
        ↓
POST /v1/sessions/{id}/messages   Spawn agent subprocess, stream output
        ↓
GET /v1/sessions/{id}/events      Poll for new events (offset-based)
GET /v1/sessions/{id}/events/sse  Subscribe to SSE stream
        ↓
POST .../questions/{id}/reply     Answer agent question
POST .../permissions/{id}/reply   Grant/deny permission request
        ↓
(agent process terminates)        Session marked as ended
```

### Event Flow

When a message is sent:

1. `send_message()` spawns the agent CLI as a subprocess
2. `consume_spawn()` reads stdout/stderr line by line
3. Each JSON line is parsed and converted via `parse_agent_line()`
4. Events are recorded via `record_event()` which:
   - Assigns incrementing event ID
   - Appends to `events` vector
   - Broadcasts to SSE subscribers

## SDK Modes

The TypeScript SDK supports two connection modes.

### Embedded Mode

Defined in `sdks/typescript/src/spawn.ts`:

1. **Binary resolution**: Checks `SANDBOX_AGENT_BIN` env, then platform-specific npm package, then `PATH`
2. **Port selection**: Uses provided port or finds a free one via `net.createServer()`
3. **Token generation**: Uses provided token or generates random 24-byte hex string
4. **Spawn**: Launches `sandbox-agent --host <host> --port <port> --token <token>`
5. **Health wait**: Polls `GET /v1/health` until server is ready (up to 15s timeout)
6. **Cleanup**: On dispose, sends SIGTERM then SIGKILL if needed; also registers process exit handlers

```typescript
const handle = await spawnSandboxDaemon({ log: "inherit" });
// handle.baseUrl = "http://127.0.0.1:<port>"
// handle.token = "<generated>"
// handle.dispose() to cleanup
```

### Server Mode

Defined in `sdks/typescript/src/client.ts`:

- Direct HTTP client to a remote `sandbox-agent` server
- Uses provided `baseUrl` and optional `token`
- No subprocess management

```typescript
const client = new SandboxDaemonClient({
  baseUrl: "http://remote-server:8080",
  token: "secret",
});
```

### Auto-Detection

`SandboxDaemonClient.connect()` chooses the mode automatically:

```typescript
// If baseUrl provided → server mode
const client = await SandboxDaemonClient.connect({
  baseUrl: "http://remote:8080",
});

// If no baseUrl → embedded mode (spawns subprocess)
const client = await SandboxDaemonClient.connect({});

// Explicit control
const client = await SandboxDaemonClient.connect({
  spawn: { enabled: true, port: 9000 },
});
```

The `spawn` option can be:
- `true` / `false` - Enable/disable embedded mode
- `SandboxDaemonSpawnOptions` - Fine-grained control over host, port, token, binary path, timeout, logging
