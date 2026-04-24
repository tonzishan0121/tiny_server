# API Conventions

## Goal

This project uses one unified JSON envelope for all chat-related API endpoints.

## Response Format

### Success

```json
{
  "ok": true,
  "code": 0,
  "message": "ok",
  "data": {}
}
```

### Error

```json
{
  "ok": false,
  "code": 1103,
  "message": "room does not exist",
  "error": {
    "code": 1103,
    "message": "room does not exist",
    "details": {
      "room": "unknown"
    }
  },
  "data": null
}
```

## Rules

1. HTTP status describes transport/result class.
2. Business code describes product/domain meaning.
3. `data` must always exist in the envelope.
4. `message` must be human-readable.
5. Validation failures should return `400`.
6. Missing resources should return `404`.
7. Duplicate-create conflicts should return `409`.

## Current API List

1. `GET /api/health`
2. `GET /api/meta`
3. `GET /api/rooms`
4. `POST /api/rooms`
5. `POST /api/rooms/rename`
6. `POST /api/rooms/delete`
7. `GET /api/messages?room=...`
8. `POST /api/messages`

## Development Requirements

1. New API endpoints must follow the same envelope.
2. New domain errors must add a business code in `doc/business_codes.md`.
3. New API endpoints must add end-to-end tests.
4. Frontend code must parse `ok/code/message/data` instead of relying on raw arrays.
