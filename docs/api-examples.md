# Grant Stream Escrow API — Usage Examples

## Overview

All escrow read responses include a `legal_hold` field.  
**Clients must check `legal_hold` before initiating any funding action.**  
If `legal_hold` is `true`, the server will reject all funding actions with `502`.

---

## 1. Read Escrow State

```bash
curl -s http://localhost:3000/escrow/escrow-abc-123
```

### Response — not held

```json
{
  "escrow_id":  "escrow-abc-123",
  "balance":    "5000000000000000000",
  "recipient":  "0xRecipientAddress",
  "status":     "active",
  "legal_hold": false
}
```

### Response — under legal hold

```json
{
  "escrow_id":  "escrow-abc-123",
  "balance":    "5000000000000000000",
  "recipient":  "0xRecipientAddress",
  "status":     "active",
  "legal_hold": true
}
```

---

## 2. Fund Escrow (blocked when held)

```bash
curl -s -X POST http://localhost:3000/escrow/escrow-abc-123/fund \
  -H "Content-Type: application/json" \
  -d '{"amount": "1000000000000000000"}'
```

### Response — success (legal_hold: false)

```json
{
  "message":   "Funding initiated",
  "escrow_id": "escrow-abc-123",
  "amount":    "1000000000000000000"
}
```

### Response — blocked (legal_hold: true) → HTTP 502

```json
{
  "error": "Escrow is under legal hold"
}
```

---

## 3. Release Escrow

```bash
curl -s -X POST http://localhost:3000/escrow/escrow-abc-123/release \
  -H "Content-Type: application/json"
```

### Response — blocked → HTTP 502

```json
{
  "error": "Escrow is under legal hold"
}
```

---

## 4. Withdraw from Escrow

```bash
curl -s -X POST http://localhost:3000/escrow/escrow-abc-123/withdraw \
  -H "Content-Type: application/json"
```

---

## Error Reference

| Status | Meaning |
|--------|---------|
| 200    | Success |
| 400    | Invalid input (bad escrow ID, missing fields) |
| 404    | Escrow not found |
| 502    | Escrow is under legal hold — action blocked |
| 503    | On-chain adapter unavailable |

---

## Security Notes

1. **Safe-fail default**: If the `legal_hold` field is missing or not a boolean in the on-chain response, the API defaults to `true` (blocked). A broken adapter cannot accidentally unblock a held escrow.

2. **No stack traces**: Error responses never include internal details or stack traces.

3. **Input validation**: Escrow IDs are validated against `/^[a-zA-Z0-9_-]{1,64}$/` before any on-chain call is made.

4. **Client responsibility**: Clients should read the escrow state first and check `legal_hold` before attempting any funding action. This avoids unnecessary 502 responses.

5. **Consistent gating**: The `legalHoldGate` middleware is applied to all mutating endpoints (`/fund`, `/release`, `/withdraw`). Adding new funding endpoints must include this middleware.

---

## OpenAPI Snippet

```yaml
paths:
  /escrow/{escrowId}:
    get:
      summary: Read escrow state
      parameters:
        - name: escrowId
          in: path
          required: true
          schema: { type: string, pattern: '^[a-zA-Z0-9_-]{1,64}$' }
      responses:
        '200':
          content:
            application/json:
              schema:
                type: object
                properties:
                  escrow_id:  { type: string }
                  balance:    { type: string }
                  recipient:  { type: string }
                  status:     { type: string }
                  legal_hold: { type: boolean }

  /escrow/{escrowId}/fund:
    post:
      summary: Fund escrow (blocked if legal_hold)
      responses:
        '200': { description: Funding initiated }
        '502': { description: Escrow is under legal hold }
```
