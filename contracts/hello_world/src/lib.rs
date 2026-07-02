#![no_std]

extern crate alloc;

#[cfg(all(not(test), target_arch = "wasm32"))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use alloc::vec::Vec;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Bytes, BytesN, Env, String,
};

const DECISION_ALLOW: u32 = 0;
const DECISION_REJECT: u32 = 1;
const DECISION_DELAY: u32 = 2;
const DECISION_PARTIAL: u32 = 3;
const MAX_DELAY_SECONDS: u64 = 7 * 24 * 60 * 60;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GuardianError {
    MissingAuthority = 1,
    InvalidDecision = 2,
    ReplayDetected = 3,
    ExpiredDecision = 4,
    InvalidDelayDuration = 5,
    DelayNotReady = 6,
    InvalidSignature = 7,
    InvalidStoredRecord = 8,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Authority,
    Decision(u64),
}

#[derive(Clone)]
#[contracttype]
pub struct DecisionRecord {
    pub decision: u32,
    pub requested_amount: i128,
    pub recipient: Address,
    pub nonce: u64,
    pub expiry_timestamp: u64,
    pub delay_seconds: u64,
    pub partial_amount: i128,
    pub approved_amount: i128,
    pub recorded_at: u64,
    pub execute_after: u64,
    pub executed_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum DecisionOutcome {
    Allowed(i128),
    Rejected,
    Delayed(u64),
    Partial(i128),
    Executed(i128),
}

#[contract]
pub struct GuardianExecutorContract;

#[contractimpl]
impl GuardianExecutorContract {
    pub fn __constructor(env: Env, authority: BytesN<32>) {
        env.storage().persistent().set(&DataKey::Authority, &authority);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Authority, 50, 1000);
    }

    pub fn execute_with_verified_decision(
        env: Env,
        decision: u32,
        amount: i128,
        recipient: Address,
        nonce: u64,
        expiry_timestamp: u64,
        delay_seconds: u64,
        partial_amount: i128,
        signature: BytesN<64>,
    ) -> Result<DecisionOutcome, GuardianError> {
        let current_timestamp = env.ledger().timestamp();

        if current_timestamp > expiry_timestamp {
            return Err(GuardianError::ExpiredDecision);
        }

        let authority = Self::trusted_authority(&env)?;
        let record_key = DataKey::Decision(nonce);

        if env.storage().persistent().has(&record_key) {
            return Err(GuardianError::ReplayDetected);
        }

        let payload = build_decision_payload(
            &env,
            decision,
            amount,
            &recipient,
            nonce,
            expiry_timestamp,
            delay_seconds,
            partial_amount,
        );

        env.crypto().ed25519_verify(&authority, &payload, &signature);

        let (approved_amount, outcome) = match decision {
            DECISION_ALLOW => (amount, DecisionOutcome::Allowed(amount)),
            DECISION_REJECT => (0, DecisionOutcome::Rejected),
            DECISION_DELAY => {
                if delay_seconds == 0 || delay_seconds > MAX_DELAY_SECONDS {
                    return Err(GuardianError::InvalidDelayDuration);
                }

                (
                    0,
                    DecisionOutcome::Delayed(current_timestamp + delay_seconds),
                )
            }
            DECISION_PARTIAL => {
                let approved = if amount < partial_amount {
                    amount
                } else {
                    partial_amount
                };

                (
                    approved,
                    DecisionOutcome::Partial(approved),
                )
            }
            _ => return Err(GuardianError::InvalidDecision),
        };

        let record = DecisionRecord {
            decision,
            requested_amount: amount,
            recipient,
            nonce,
            expiry_timestamp,
            delay_seconds,
            partial_amount,
            approved_amount,
            recorded_at: current_timestamp,
            execute_after: if decision == DECISION_DELAY {
                current_timestamp + delay_seconds
            } else {
                0
            },
            executed_at: if decision == DECISION_REJECT {
                current_timestamp
            } else {
                0
            },
        };

        env.storage().persistent().set(&record_key, &record);
        env.storage().persistent().extend_ttl(&record_key, 50, 1000);

        Ok(outcome)
    }

    pub fn execute_delayed_transaction(
        env: Env,
        nonce: u64,
        recipient: Address,
    ) -> Result<DecisionOutcome, GuardianError> {
        let record_key = DataKey::Decision(nonce);
        let mut record: DecisionRecord = env
            .storage()
            .persistent()
            .get(&record_key)
            .ok_or(GuardianError::InvalidStoredRecord)?;

        if record.decision != DECISION_DELAY || record.recipient != recipient {
            return Err(GuardianError::InvalidStoredRecord);
        }

        let current_timestamp = env.ledger().timestamp();
        if current_timestamp < record.execute_after {
            return Err(GuardianError::DelayNotReady);
        }

        record.executed_at = current_timestamp;
        env.storage().persistent().set(&record_key, &record);
        env.storage().persistent().extend_ttl(&record_key, 50, 1000);

        Ok(DecisionOutcome::Executed(record.approved_amount))
    }

    pub fn get_decision(env: Env, nonce: u64) -> Option<DecisionRecord> {
        env.storage().persistent().get(&DataKey::Decision(nonce))
    }

    fn trusted_authority(env: &Env) -> Result<BytesN<32>, GuardianError> {
        env.storage()
            .persistent()
            .get(&DataKey::Authority)
            .ok_or(GuardianError::MissingAuthority)
    }
}

fn build_decision_payload(
    env: &Env,
    decision: u32,
    amount: i128,
    recipient: &Address,
    nonce: u64,
    expiry_timestamp: u64,
    delay_seconds: u64,
    partial_amount: i128,
) -> Bytes {
    let mut payload = Vec::<u8>::new();

    payload.extend_from_slice(&decision.to_le_bytes());
    payload.extend_from_slice(&amount.to_le_bytes());

    let recipient_string: String = recipient.to_string();
    let mut recipient_bytes = Vec::<u8>::new();
    recipient_bytes.resize(recipient_string.len() as usize, 0);
    recipient_string.copy_into_slice(&mut recipient_bytes);
    payload.extend_from_slice(&recipient_bytes);

    payload.extend_from_slice(&nonce.to_le_bytes());
    payload.extend_from_slice(&expiry_timestamp.to_le_bytes());
    payload.extend_from_slice(&delay_seconds.to_le_bytes());
    payload.extend_from_slice(&partial_amount.to_le_bytes());

    Bytes::from_slice(env, &payload)
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn builds_payload_deterministically() {
        let env = Env::default();
        let recipient = Address::generate(&env);
        let payload_one = build_decision_payload(
            &env,
            DECISION_ALLOW,
            1_000,
            &recipient,
            7,
            10_000,
            0,
            500,
        );
        let payload_two = build_decision_payload(
            &env,
            DECISION_ALLOW,
            1_000,
            &recipient,
            7,
            10_000,
            0,
            500,
        );

        assert_eq!(payload_one, payload_two);
        assert!(!payload_one.is_empty());
    }
}