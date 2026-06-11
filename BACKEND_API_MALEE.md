# Malee AI Shopping Guide - Backend API Documentation

This document describes the API endpoints for the **Malee (මලී)** feature, a Sri Lankan AI shopping agent.

## 1. General Information

- **Base URL**: `/api/v1/malee`
- **Authentication**: All requests require the `x-api-key` header.
- **Content-Type**: `application/json` (except for the chat endpoint which returns `text/event-stream`).
- **Intelligence**: Powered by a pooled LLM system supporting Groq, Google Gemini, Cerebras, and Fireworks for high-speed, reliable Sri Lankan e-commerce assistance.

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
    - `set_gift_note`: Payload `{ "note": "string" }`.
    - `set_language`: Payload `{ "mode": "string" }`.
- **Response**: `200 OK` with updated `CartView`.

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
| `checkout_form` | `{ "draft": {...}, "missing_fields": ["phone"] }` | Current checkout state and missing fields. |
| `checkout_ready` | `{ "pay_url": "...", "order_ref": "...", "expires_in_minutes": 15, "cart_summary": [...] }` | Payment link is ready. |
| `question_prompt` | `{ "questions": [{ "field": "delivery_date", "label": "...", "input_type": "date" }] }` | Optimized input prompt for one or more fields. |
| `tracking_result` | `{ "order_number": "...", "status": "Shipped", ... }` | Result of an order tracking request. |
| `language_changed` | `{ "mode": "sinhala" }` | Emitted when language mode is switched. |
| `error` | `{ "code": "LOOP_DEPTH", "message": "...", "recoverable": true }` | Error details. |

## 5. Data Models (JSON)

### ProductCardView
```json
{
  "id": "string",
  "name": "string",
  "price_lkr": number,
  "image_url": "string | null",
  "in_stock": boolean
}
```

### ProductDetailView
```json
{
  "id": "string",
  "name": "string",
  "description": "string | null",
  "price_lkr": number,
  "image_urls": ["string"],
  "in_stock": boolean,
  "is_perishable": boolean,
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
      "price_lkr": number,
      "quantity": number,
      "image_url": "string | null"
    }
  ],
  "subtotal_lkr": number,
  "item_count": number
}
```

### CheckoutDraftView
```json
{
  "recipient_name": "string | null",
  "delivery_city": "string | null",
  "delivery_date": "string | null",
  "sender_name": "string | null",
  "gift_message": "string | null"
}
```
