# Rastraq

Rastraq is a personal daily radar for newly published technical information.

Phase 1 intentionally does not crawl the web or schedule external agents. An
agent, script, or operator submits candidate items. Rastraq stores them,
generates deterministic mock summaries and embeddings, ranks the previous local
day, freezes a fixed daily edition, and records feedback for future ranking.

## Stack

- Backend: Rust, axum, sqlx, SQLite
- Frontend: lit, Vite, TypeScript
- Default user timezone: `Asia/Tokyo`
- Default daily item count: `5`
- LLM provider: `DeterministicMockProvider` behind a provider trait

## Run

```bash
cargo run
```

The server listens on `127.0.0.1:3000` by default and stores SQLite data at
`data/rastraq.sqlite`.

Environment variables:

```bash
RASTRAQ_ADDR=127.0.0.1:3000
RASTRAQ_DATABASE_URL=sqlite://data/rastraq.sqlite?mode=rwc
```

Build the frontend when Node/npm is available:

```bash
cd web
npm install
npm run build
```

## Agent-Friendly Flow

Submit candidates discovered by an external agent:

```bash
curl -X POST http://127.0.0.1:3000/api/items \
  -H 'content-type: application/json' \
  -d '{
    "url": "https://example.com/rust-security",
    "title": "Rust security advisory",
    "source_type": "security_advisory",
    "published_at": "2026-04-23T03:00:00Z",
    "raw_content": "Rust crate maintainers published a security fix and release notes."
  }'
```

Process an item with the mock LLM provider:

```bash
curl -X POST http://127.0.0.1:3000/api/items/1/process \
  -H 'content-type: application/json' \
  -d '{}'
```

Freeze the previous local day into a fixed daily edition:

```bash
curl -X POST http://127.0.0.1:3000/api/editions/generate \
  -H 'content-type: application/json' \
  -d '{"now":"2026-04-24T01:00:00Z"}'
```

Read today’s edition:

```bash
curl http://127.0.0.1:3000/api/editions/today
```

Record feedback:

```bash
curl -X POST http://127.0.0.1:3000/api/feedback \
  -H 'content-type: application/json' \
  -d '{"item_id":1,"event_type":"interested","payload":{"surface":"daily-card"}}'
```

Fetch interest keywords for a search agent:

```bash
curl http://127.0.0.1:3000/api/interest-keywords
```

## API Summary

- `POST /api/items`
- `POST /api/items/{id}/process`
- `POST /api/editions/generate`
- `GET /api/editions/today`
- `GET /api/editions?date=YYYY-MM-DD`
- `POST /api/feedback`
- `GET /api/interest-keywords`
- `GET /api/health`

## TDD Verification

Implemented with the `canon-tdd` workflow:

- Timezone previous-day behavior
- Deterministic mock summary and embedding
- Explicit ranking with saved feature signals
- Full API flow from item submission to daily edition and feedback

Run:

```bash
cargo test
cargo build
```
