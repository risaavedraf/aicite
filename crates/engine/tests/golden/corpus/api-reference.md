# CloudUsers API Reference

**Version:** 2.4.1
**Base URL:** `https://api.cloudusers.io/v2`
**Authentication:** Bearer token via `Authorization` header

---

## Overview

The CloudUsers API provides programmatic access to user management, team
organization, and role-based access control. All requests must include a valid
API key passed as a Bearer token. Responses are JSON-encoded and follow
standard HTTP status conventions.

---

## Authentication

Every request requires an `Authorization` header:

```
Authorization: Bearer <your_api_key>
```

API keys are scoped to an organization and can be rotated from the admin
dashboard. Keys grant access to all endpoints unless restricted by role
permissions at the organization level.

---

## Rate Limiting

Rate limiting is enforced per API key. Each key is allowed **100 requests per
minute**, with a burst allowance of **10 concurrent requests**. Exceeding
either limit will result in a `429 Too Many Requests` response.

When you receive a 429 response, consult the `Retry-After` header to determine
how many seconds to wait before retrying. The header value is an integer
representing seconds.

| Header             | Description                              |
| ------------------ | ---------------------------------------- |
| `X-RateLimit-Limit`     | Maximum requests allowed per minute |
| `X-RateLimit-Remaining` | Requests remaining in current window |
| `X-RateLimit-Reset`     | UTC epoch timestamp when the window resets |
| `Retry-After`           | Seconds to wait before retrying (on 429) |

---

## Endpoints

### List Users

Retrieves a paginated list of users within the authenticated organization.

```
GET /users?page=1&per_page=20
```

**Query Parameters:**

| Parameter  | Type    | Default | Max  | Description                     |
| ---------- | ------- | ------- | ---- | ------------------------------- |
| `page`     | integer | 1       | —    | Page number to retrieve         |
| `per_page` | integer | 20      | 100  | Number of users per page        |
| `role`     | string  | —       | —    | Filter by role (`admin`, `member`, `viewer`) |
| `status`   | string  | —       | —    | Filter by status (`active`, `suspended`, `invited`) |

The endpoint returns a paginated list of users with a **default page size of
20** and a **maximum page size of 100**. The response includes pagination
metadata.

**Response (200):**

```json
{
  "data": [
    {
      "id": "usr_9f3a2b",
      "email": "jane@example.com",
      "role": "admin",
      "status": "active",
      "created_at": "2025-11-03T14:22:00Z"
    }
  ],
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 87,
    "total_pages": 5
  }
}
```

---

### Create User

Creates a new user and sends an invitation email.

```
POST /users
```

**Request Body:**

The request body must include both `email` and `role` fields. The `email`
field must be a valid email address, and the `role` field must be one of
`admin`, `member`, or `viewer`.

```json
{
  "email": "newuser@example.com",
  "role": "member",
  "name": "New User"
}
```

**Required Fields:**

| Field   | Type   | Description                                     |
| ------- | ------ | ----------------------------------------------- |
| `email` | string | Email address for the new user (required)       |
| `role`  | string | Role assignment: `admin`, `member`, or `viewer` (required) |
| `name`  | string | Display name (optional)                         |

**Response (201):**

On success the endpoint returns HTTP 201 with the created user ID.

```json
{
  "id": "usr_4c7d1e",
  "email": "newuser@example.com",
  "role": "member",
  "status": "invited",
  "created_at": "2026-01-15T09:00:00Z"
}
```

---

### Get User

```
GET /users/{user_id}
```

Returns full details for a single user by ID.

**Response (200):**

```json
{
  "id": "usr_9f3a2b",
  "email": "jane@example.com",
  "role": "admin",
  "status": "active",
  "teams": ["engineering", "platform"],
  "last_login": "2026-05-26T17:45:00Z",
  "created_at": "2025-11-03T14:22:00Z"
}
```

---

### Update User

```
PATCH /users/{user_id}
```

Partially updates a user record. Only the fields included in the request body
are modified.

---

### Delete User

```
DELETE /users/{user_id}
```

Permanently removes a user. Returns `204 No Content` on success. This action
cannot be undone.

---

## Error Codes

| Code | Meaning              | Description                                                    |
| ---- | -------------------- | -------------------------------------------------------------- |
| 400  | Bad Request          | Malformed JSON or missing required fields                      |
| 401  | Unauthorized         | Invalid or missing API key                                     |
| 403  | Forbidden            | Insufficient permissions for the requested action              |
| 404  | Not Found            | The requested resource does not exist                           |
| 409  | Conflict             | Resource already exists (e.g., duplicate email)                |
| 429  | Rate Limit Exceeded  | Too many requests; wait for the `Retry-After` header before retrying |
| 500  | Internal Server Error | Unexpected server failure; contact support                    |

---

## Pagination

All list endpoints return paginated results. Use the `page` and `per_page`
parameters to navigate through result sets. The response `meta` object
includes `total`, `page`, `per_page`, and `total_pages` fields to aid
client-side pagination logic.

---

## Webhooks

CloudUsers can send webhook events for user lifecycle changes. Register a
webhook URL in the admin dashboard. All webhook payloads include a signature
header `X-CloudUsers-Signature` for verification.

Supported events: `user.created`, `user.updated`, `user.deleted`,
`user.role_changed`, `user.status_changed`.
