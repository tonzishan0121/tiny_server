# Business Codes

## Success

| Code | Name | Meaning |
|---|---|---|
| `0` | `OK` | Request succeeded |

## Generic Request Errors

| Code | Name | Meaning |
|---|---|---|
| `1000` | `BAD_REQUEST` | Generic request or parameter error |

## Room Errors

| Code | Name | Meaning |
|---|---|---|
| `1101` | `ROOM_INVALID` | Room name is invalid |
| `1102` | `ROOM_ALREADY_EXISTS` | Room already exists |
| `1103` | `ROOM_NOT_FOUND` | Room does not exist |
| `1104` | `ROOM_PROTECTED` | Room cannot be updated or deleted |

## User Errors

| Code | Name | Meaning |
|---|---|---|
| `1201` | `USER_INVALID` | User name is invalid |

## Message Errors

| Code | Name | Meaning |
|---|---|---|
| `1301` | `MESSAGE_INVALID` | Message content is invalid |

## Internal Errors

| Code | Name | Meaning |
|---|---|---|
| `9000` | `INTERNAL_ERROR` | Unexpected server-side error |

## Maintenance Rule

1. Every new business code must be added here first.
2. One code should represent one stable business meaning.
3. Do not reuse an old code for a new meaning.
