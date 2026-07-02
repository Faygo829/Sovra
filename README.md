# Guardian Executor on Stellar

Guardian Executor is a Stellar-first transaction protection system. It combines local AI, signed policy decisions, and Soroban enforcement to help users decide whether a payment should be allowed, delayed, reduced, or blocked.

CONTRACT ADDRESS : CAEIIWEKVHLOPNRIFT4HW2Q3PUSAQGZ3REKAX4GAXXPIEVAEYESN4B33

## What it is

Most wallet flows stop at a basic signature prompt. Guardian adds a decision layer before the transaction is finalized:

1. A transaction is analyzed against the user’s normal behavior.
2. A deterministic policy decision is produced.
3. That decision is signed so it cannot be altered later.
4. The Stellar-side enforcement layer verifies the signed decision.
5. The final result is executed or blocked on-chain.

The goal is simple: keep Stellar transfers fast and low-cost, but add a strong trust-and-safety layer for payments that look unusual, risky, or high value.

## Why Stellar

Stellar is a strong fit for this idea because it is designed for payments, settlement, and predictable transaction flows. Soroban makes it possible to add programmable guardrails without changing the user’s core payment experience.

This makes the system useful for:

- consumer wallet protection
- treasury approval workflows
- payroll and payout controls
- staged approvals for large transfers
- fraud and phishing defense

## How the flow works

1. The client prepares a Stellar transaction.
2. The local decision engine scores the transfer.
3. The resulting policy is signed by a trusted authority.
4. The transaction carries the signed policy data.
5. The Soroban contract verifies the policy and applies the rule.
6. The final action is recorded for auditability.

## Decision types

### Allow

The transfer is considered safe and proceeds normally.

### Reject

The transfer is considered too risky and is blocked.

### Delay

The transfer is held for a review window or until a future time.

### Partial

Only part of the requested amount is approved, reducing exposure while still allowing movement of funds.

## Security model

The design focuses on preventing tampering and replay:

- The signed payload includes the amount, destination, nonce, and expiry.
- The contract verifies that the signature came from the trusted authority.
- Expiry rules stop stale approvals from being reused.
- Nonces stop the same approval from being replayed twice.

## Repository layout

- `programs/guardian_executor/` - on-chain enforcement logic
- `guardianClient.ts` - client-side transaction assembly and signing
- `qvac-decision-engine/` - local decision pipeline and validation tooling
- `mobile-app/` - user-facing app shell
- `contracts/hello_world/` - minimal Soroban contract scaffold for Stellar testnet submissions

## Stellar testnet setup

- Connect a Stellar wallet through Freighter in the mobile app.
- The app reads the active testnet contract from `EXPO_PUBLIC_STELLAR_CONTRACT_ID`.
- Deploy the Soroban scaffold in `contracts/hello_world/` to Stellar testnet and set `EXPO_PUBLIC_STELLAR_CONTRACT_ID` to your deployed contract ID (for example, `CC...` on testnet).
- A starter env template is available in `mobile-app/.env.example`.

## Core idea

Guardian is not trying to replace Stellar. It adds a policy layer on top of Stellar transfers so users can make smarter payment decisions without giving up the chain’s speed, simplicity, or low fees.

In short: AI evaluates intent, cryptography locks the decision, and Soroban enforces the result.

## License

MIT

**Built for:** Stellar • Soroban • Rust • TypeScript • Local AI
