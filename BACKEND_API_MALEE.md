# Malee AI Shopping Guide - Backend API Documentation

This document describes the API endpoints for the **Malee (මලී)** feature, a Sri Lankan AI shopping agent. This is the **100% accurate contract document** matching the backend implementation.

## 1. General Information

- **Base URL**: `/api/v1/malee`
- **Authentication**: All requests require the `x-api-key` header.
- **Content-Type**: `application/json` (except for the chat endpoint which returns `text/event-stream`).
- **Intelligence**: Powered by a pooled LLM router supporting an `OpenAiCompatibleClient` wrapper for models from Groq, Cerebras, Fireworks, and Ollama.
- **Resilience**: Features automatic multi-provider failover, `45s` chunk stream timeouts, and robust parsing to automatically recover from `RATE_LIMIT`, `5xx`, and malformed JSON errors without dropping user sessions.

## 2. Authentication

Include your master API key in the headers for every request:

```http
x-api-key: YOUR_MASTER_API_KEY
```

## 3. Endpoints

### 3.1 Conversational Chat (SSE)
Conversational interface with Malee. This endpoint uses Server-Sent Events (SSE) to stream tokens and UI events.

- **URL**: `POST /chat`
- **Payload**:
```json
{
  "message": "I want to buy a gift for my mother",
  "session_id": "optional-uuid-string",
  "language_mode": "auto" 
}
```
- **Language Modes**: `auto`, `english`, `sinhala`, `mixed`.
- **Response**: `text/event-stream`
- **Event Types**: See [Section 4: SSE Event Protocol](#4-sse-event-protocol).

---

### 3.2 Update Session Action
Synchronous endpoint to update session state (cart, delivery info, etc.) without involving the LLM.

- **URL**: `POST /action`
- **Payload**:
```json
{
  "session_id": "uuid-string",
  "action": "action_name",
  "payload": { ... }
}
```
- **Supported Actions**:
    - `add_to_cart`: Payload is a `CartItem` object.
    - `remove_from_cart`: Payload `{ "product_id": "string" }`.
    - `set_quantity`: Payload `{ "product_id": "string", "quantity": number }`.
    - `clear_cart`: No payload.
    - `set_delivery_city`: Payload `{ "city": "string" }`.
    - `set_delivery_date`: Payload `{ "date": "YYYY-MM-DD" }`.
    - `set_gift_note`: Payload `{ "note": "string" }`. (Max 240 chars)
    - `set_language`: Payload `{ "mode": "string" }` (auto, english, sinhala, mixed).
- **Response**: `200 OK` with updated `CartView` (See Section 5).

---

### 3.3 Get Session Summary
Retrieve the current state of a session (cart, checkout draft, etc.).

- **URL**: `GET /session/{id}`
- **Response**: `200 OK`
```json
{
  "session_id": "uuid",
  "language_mode": "auto",
  "cart": { ... },
  "checkout_draft": { ... },
  "last_product_ids": ["id1", "id2"]
}
```

---

### 3.4 Reset Session
Permanently delete a session and its history.

- **URL**: `DELETE /session/{id}`
- **Response**: `204 No Content`

---

### 3.5 Track Order
Directly track an order without conversation history.

- **URL**: `POST /track`
- **Payload**: `{ "order_number": "string" }`
- **Response**: `200 OK`
```json
{
  "order_number": "string",
  "status": "string",
  "recipient": "string",
  "items": ["item1", "item2"],
  "timeline": [
    { "timestamp": "ISO-8601", "description": "string" }
  ]
}
```

---

### 3.6 Get User Profile
Retrieve the user profile for a specific session.

- **URL**: `GET /session/{id}/profile`
- **Response**: `200 OK`
```json
{
  "session_id": "uuid",
  "profile": { ... } // See UserProfile in Section 5
}
```

---

### 3.7 Update User Profile
Update the user profile for a specific session.

- **URL**: `PUT /session/{id}/profile`
- **Payload**:
```json
{
  "profile": { ... } // See UserProfile in Section 5
}
```
- **Response**: `200 OK` with the updated profile object.
```json
{
  "session_id": "uuid",
  "profile": { ... }
}
```

## 4. SSE Event Protocol

The `/chat` endpoint streams lines prefixed with `data: `. Each line is a JSON object with a `type` field.

| Event Type | Data Payload Example | Description |
|:---|:---|:---|
| `session_created` | `{ "session_id": "uuid" }` | Emitted when a new session is started. |
| `token` | `{ "text": "token" }` | Incremental chat token (for typing effect). |
| `assistant_message_done` | `{ "full_text": "full text" }` | Final accumulated assistant message. |
| `product_carousel` | `{ "title": "Lily Gifts", "subtitle": "...", "items": [...] }` | List of products to display as cards. |
| `product_detail` | `{ "item": {...} }` | Full details for a single product. |
| `category_grid` | `{ "categories": [...] }` | Product categories for navigation. |
| `cart_updated` | `{ "cart": {...} }` | Triggered when the agent modifies the cart. |
| `city_suggestions` | `{ "query": "Colo", "cities": ["Colombo"] }` | List of deliverable cities. |
| `delivery_quote` | `{ "city": "...", "date": "...", "rate_lkr": 500, "deliverable": true, "perishable_warning": false, "next_available_date": null }` | Shipping costs and feasibility. |
| `checkout_progress` | `{ "current_step": 2, "total_steps": 4, "step_name": "Recipient Details", "missing_fields": ["phone"] }` | Active checkout step indication. |
| `checkout_form` | `{ "draft": {...}, "missing_fields": ["phone"] }` | Current checkout state and missing fields. |
| `checkout_ready` | `{ "pay_url": "...", "order_ref": "...", "expires_in_minutes": 60, "cart_summary": [...] }` | Payment link is ready. |
| `question_prompt` | `{ "questions": [{ "field": "delivery_date", "label": "...", "input_type": "date", "placeholder": null }] }` | Optimized input prompt for one or more fields. |
| `tracking_result` | `{ "order_number": "...", "status": "Shipped", ... }` | Result of an order tracking request. |
| `language_changed` | `{ "mode": "sinhala" }` | Emitted when language mode is switched. |
| `error` | `{ "code": "LOOP_DEPTH", "message": "...", "recoverable": true }` | Error details. |

## 5. Data Models (JSON)

### ProductCardView
```json
{
  "id": "string",
  "name": "string",
  "price_lkr": 1500,
  "image_url": "string | null",
  "in_stock": true
}
```

### ProductDetailView
```json
{
  "id": "string",
  "name": "string",
  "description": "string | null",
  "price_lkr": 1500,
  "image_urls": ["string"],
  "in_stock": true,
  "is_perishable": false,
  "vendor_name": "string | null"
}
```

### CartView
```json
{
  "items": [
    {
      "product_id": "string",
      "name": "string",
      "price_lkr": 1500,
      "quantity": 1,
      "image_url": "string | null"
    }
  ],
  "subtotal_lkr": 1500,
  "item_count": 1
}
```

### CheckoutDraftView
```json
{
  "recipient_name": "string | null",
  "recipient_address": "string | null",
  "delivery_city": "string | null",
  "delivery_date": "string | null",
  "sender_name": "string | null",
  "gift_message": "string | null"
}
```

### UserProfile
```json
{
  "first_name": "string | null",
  "last_name": "string | null",
  "email": "string | null",
  "phone": "string | null",
  "address_line1": "string | null",
  "address_line2": "string | null",
  "city": "string | null",
  "zip_code": "string | null",
  "currency": "string | null",
  "preferred_language": "string | null",
  "favorite_categories": ["string"],
  "memories": ["string"],
  "order_history": [
    {
      "order_ref": "string",
      "date": "ISO-8601 string",
      "items": ["string"],
      "total_lkr": 1500
    }
  ]
}
```
