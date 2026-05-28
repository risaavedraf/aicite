# API Reference

This document describes the REST API endpoints for the user management service.

## Users

### GET /users

Returns a paginated list of users. Default page size is 20, maximum is 100. Supports filtering by role and status.

**Query parameters:**
- `page` (integer, default: 1) — Page number
- `per_page` (integer, default: 20, max: 100) — Items per page
- `role` (string, optional) — Filter by role: `admin`, `member`, `viewer`
- `status` (string, optional) — Filter by status: `active`, `inactive`, `suspended`

**Response (200):**
```json
{
  "data": [{ "id": "usr_abc123", "email": "user@example.com", "role": "member", "status": "active" }],
  "pagination": { "page": 1, "per_page": 20, "total": 142 }
}
```

### POST /users

Creates a new user account. Requires `email` and `role` fields in the request body. Returns 201 with the created user ID on success.

**Request body:**
```json
{
  "email": "newuser@example.com",
  "role": "member"
}
```

**Response (201):**
```json
{
  "id": "usr_def456",
  "email": "newuser@example.com",
  "role": "member",
  "status": "active",
  "created_at": "2024-01-15T10:30:00Z"
}
```

## Error Handling

All errors follow a consistent format with `code`, `message`, and optional `details` fields.

### Rate Limiting

Error code 429 means rate limit exceeded. Clients should wait for the `Retry-After` header value (in seconds) before retrying. Rate limiting is set to 100 requests per minute per API key, with a burst allowance of 10 concurrent requests per key.

### Common Error Codes

| Code | Meaning |
|---|---|
| 400 | Bad request — invalid input |
| 401 | Unauthorized — missing or invalid credentials |
| 403 | Forbidden — insufficient permissions |
| 404 | Not found — resource does not exist |
| 429 | Too many requests — rate limit exceeded |
| 500 | Internal server error |
