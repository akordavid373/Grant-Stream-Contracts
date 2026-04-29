/**
 * legalHoldGate.js
 * ────────────────
 * Express middleware that blocks any funding action when the escrow is under
 * legal hold.
 *
 * Usage:
 *   router.post("/fund", legalHoldGate, fundHandler);
 *
 * Flow:
 *   1. Read `escrow_id` from `req.params`, `req.body`, or `req.query`.
 *   2. Fetch escrow state via `readEscrow`.
 *   3. If `legal_hold === true` → respond 502 and stop the chain.
 *   4. Otherwise attach `req.escrow` for downstream handlers and call next().
 *
 * Security:
 *  - Returns 502 (Bad Gateway) to signal that the upstream on-chain state
 *    prevents the action — consistent with the spec.
 *  - No stack traces or internal details are leaked to the client.
 *  - Defaults to blocking (502) on any unexpected error reading escrow state,
 *    so a broken adapter cannot accidentally allow a held escrow through.
 */

"use strict";

const { readEscrow } = require("../services/escrowRead");

/**
 * Middleware factory — returns a configured middleware function.
 * Exported directly as the default middleware (no config needed for now).
 */
async function legalHoldGate(req, res, next) {
  // Resolve escrow ID from the most common locations
  const escrowId =
    req.params?.escrowId ||
    req.body?.escrow_id  ||
    req.query?.escrow_id;

  if (!escrowId) {
    return res.status(400).json({ error: "Missing escrow_id" });
  }

  let escrow;
  try {
    escrow = await readEscrow(escrowId);
  } catch (err) {
    // Propagate validation errors (400) and not-found (404) as-is
    if (err.statusCode === 400 || err.statusCode === 404) {
      return res.status(err.statusCode).json({ error: err.message });
    }
    // Any other failure (adapter down, unexpected data) → block safely
    return res.status(502).json({ error: "Escrow is under legal hold" });
  }

  if (escrow.legal_hold === true) {
    return res.status(502).json({ error: "Escrow is under legal hold" });
  }

  // Attach for downstream handlers so they don't need to re-fetch
  req.escrow = escrow;
  return next();
}

module.exports = legalHoldGate;
