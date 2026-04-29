/**
 * onChainAdapter.js
 * ─────────────────
 * Thin abstraction over the on-chain data source.
 *
 * In production this would call an RPC node / indexer / subgraph.
 * The interface is kept minimal so it can be easily mocked in tests.
 *
 * Methods:
 *   getEscrow(escrowId)  → raw escrow object  (or null if not found)
 *   getLegalHold(escrowId) → boolean
 */

"use strict";

const onChainAdapter = {
  /**
   * Fetch raw escrow data from the on-chain source.
   * @param {string} escrowId
   * @returns {Promise<object|null>}
   */
  async getEscrow(escrowId) { // eslint-disable-line no-unused-vars
    // TODO: replace with actual RPC / subgraph call
    throw new Error("onChainAdapter.getEscrow not implemented");
  },

  /**
   * Fetch only the legal-hold flag (lighter call for gating checks).
   * @param {string} escrowId
   * @returns {Promise<boolean>}
   */
  async getLegalHold(escrowId) { // eslint-disable-line no-unused-vars
    // TODO: replace with actual on-chain call to get_legal_hold()
    throw new Error("onChainAdapter.getLegalHold not implemented");
  },
};

module.exports = { onChainAdapter };
