# Nadzu API

## Overview

The **Nadzu API** is a Rust-based backend service that provides health checking, user validation, and a YT-DLP media download pipeline. The download pipeline supports enqueueing video download jobs, querying job status, streaming real-time progress via Server-Sent Events (SSE), and downloading completed files.

All requests are made against a configurable base URL stored in the `{{base_url}}` variable. Authenticated endpoints require an API key passed via the `x-api-key` header.

---

## Variables

| Variable | Description |
|---|---|
| `{{base_url}}` | Base URL of the API server (e.g. `https://api.example.com`) |
| `{{api_key}}` | API key for authenticated endpoints |
| `{{captcha_token}}` | Captcha token required by the YT-DLP enqueue endpoint |
| `{{video_url}}` | URL of the video to download |
| `{{latest_job_id}}` | Set automatically after a successful enqueue; used in job-specific endpoints |

---

## Endpoints

### 1. Root

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/health` |

Performs a basic liveness check against the root health endpoint to confirm the server is reachable and responding.

#### Tests

- `Status is 200`

---

### 2. Health

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/health` |

Returns a structured health status response. Unlike the root check, this endpoint is expected to return a JSON body containing a `status` field, making it suitable for readiness probes.

#### Tests

- `Status is 200`
- `Response has status field` — verifies that `jsonData.status` exists in the response body

---

### 3. Validate User

| | |
|---|---|
| **Method** | `POST` |
| **URL** | `{{base_url}}/validate-user` |

Validates a user payload against the server-side validation rules. Useful for testing input validation logic (e.g. required fields, email format, age constraints).

#### Headers

| Key | Value |
|---|---|
| `Content-Type` | `application/json` |

#### Request Body

```json
{
  "name": "Alice",
  "email": "alice@example.com",
  "age": 24
}
```

#### Tests

- `Status is 200`
- `Validation success true` — verifies that `jsonData.success === true`

---

### 4. YT-DLP Enqueue

| | |
|---|---|
| **Method** | `POST` |
| **URL** | `{{base_url}}/api/v1/ytdlp` |

Enqueues a new video download job. The server processes the request asynchronously and returns a job object containing the assigned job ID. The `latest_job_id` collection variable is automatically set from the response for use in subsequent job-related requests.

#### Headers

| Key | Value |
|---|---|
| `Content-Type` | `application/json` |
| `x-captcha-token` | `{{captcha_token}}` |
| `x-api-key` | `{{api_key}}` |

#### Request Body

```json
{
  "url": "{{video_url}}",
  "quality": "best",
  "format": "mp4"
}
```

#### Tests

- `Status is 202` — confirms the job was accepted for async processing
- `Response has job id` — verifies that `jsonData.job.id` exists

> **Side effect:** On success, the collection variable `latest_job_id` is set to `jsonData.job.id`.

---

### 5. Supported Sites

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/api/v1/ytdlp/sites` |

Returns the list of websites and platforms supported by the YT-DLP integration.

#### Headers

| Key | Value |
|---|---|
| `x-bypass-dev` | `"true"` |

#### Tests

- `Status is 200`
- `Response has sites array` — verifies that `jsonData.sites` is an array

---

### 6. YT-DLP List Jobs

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/api/v1/ytdlp/jobs` |

Retrieves a list of all download jobs. Requires API key authentication.

#### Headers

| Key | Value |
|---|---|
| `x-api-key` | `{{api_key}}` |

#### Tests

- `Status is 200`
- `Response has jobs array` — verifies that `jsonData.jobs` is an array

---

### 7. YT-DLP Get Job By ID

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/api/v1/ytdlp/jobs/{{latest_job_id}}` |

Fetches the details and current status of a specific download job identified by `{{latest_job_id}}`.

#### Tests

- `Status is 200`
- `Response has job object` — verifies that `jsonData.job` exists

---

### 8. YT-DLP Stream Job Progress

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/api/v1/ytdlp/jobs/{{latest_job_id}}/stream` |

Streams real-time download progress for a job using Server-Sent Events (SSE). The client receives a continuous stream of progress events until the job completes or fails.

#### Headers

| Key | Value |
|---|---|
| `Accept` | `text/event-stream` |
| `x-bypass-dev` | `"true"` |

#### Tests

- `Status is 200`
- `Content-Type is text/event-stream` — verifies the response uses the SSE content type

---

### 9. YT-DLP Download File

| | |
|---|---|
| **Method** | `GET` |
| **URL** | `{{base_url}}/api/v1/ytdlp/download/{{latest_job_id}}` |

Downloads the completed media file for the given job ID. Should only be called after the job has reached a completed state (confirmed via the job status or stream endpoints).

#### Tests

- `Status is 200`

---

## Test Scripts

The following table summarises all automated tests run per endpoint:

| Endpoint | Tests |
|---|---|
| **Root** | `Status is 200` |
| **Health** | `Status is 200`, `Response has status field` |
| **Validate User** | `Status is 200`, `Validation success true` |
| **YT-DLP Enqueue** | `Status is 202`, `Response has job id` |
| **Supported Sites** | `Status is 200`, `Response has sites array` |
| **YT-DLP List Jobs** | `Status is 200`, `Response has jobs array` |
| **YT-DLP Get Job By ID** | `Status is 200`, `Response has job object` |
| **YT-DLP Stream Job Progress** | `Status is 200`, `Content-Type is text/event-stream` |
| **YT-DLP Download File** | `Status is 200` |
