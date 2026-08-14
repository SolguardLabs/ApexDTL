use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AccountId, AccountState, Amount, ApexError, ApexResult, AssetId, Digest, IntentId,
    JournalEntry, JournalOp, PublicIdentity, SignedIntent, SignedSettlement, TxId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentState {
    pub id: IntentId,
    pub terms: crate::IntentTerms,
    pub route: crate::RoutePlan,
    pub settled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ApexLedger {
    network_id: u32,
    asset: AssetId,
    current_epoch: u64,
    accounts: BTreeMap<AccountId, AccountState>,
    intents: BTreeMap<IntentId, IntentState>,
    seen_transactions: BTreeSet<TxId>,
    total_supply: Amount,
    journal: Vec<JournalEntry>,
}

impl ApexLedger {
    pub fn new(network_id: u32, asset: AssetId) -> Self {
        Self {
            network_id,
            asset,
            current_epoch: 0,
            accounts: BTreeMap::new(),
            intents: BTreeMap::new(),
            seen_transactions: BTreeSet::new(),
            total_supply: Amount::zero(),
            journal: Vec::new(),
        }
    }

    pub fn network_id(&self) -> u32 {
        self.network_id
    }

    pub fn asset(&self) -> AssetId {
        self.asset
    }

    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    pub fn set_epoch(&mut self, epoch: u64) -> ApexResult<()> {
        if epoch < self.current_epoch {
            return Err(ApexError::State(
                "ledger epoch cannot move backwards".to_owned(),
            ));
        }
        self.current_epoch = epoch;
        Ok(())
    }

    pub fn advance_epoch(&mut self, delta: u64) -> ApexResult<u64> {
        self.current_epoch = self
            .current_epoch
            .checked_add(delta)
            .ok_or(ApexError::EpochOverflow)?;
        Ok(self.current_epoch)
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    pub fn intent_count(&self) -> usize {
        self.intents.len()
    }

    pub fn register_account(&mut self, identity: PublicIdentity) -> ApexResult<()> {
        identity.verify_consistency()?;
        if self.accounts.contains_key(&identity.account) {
            return Err(ApexError::AccountAlreadyExists(identity.account));
        }
        self.accounts
            .insert(identity.account, AccountState::new(identity));
        Ok(())
    }

    pub fn credit_genesis(&mut self, account: AccountId, amount: Amount) -> ApexResult<TxId> {
        self.ensure_account(account)?;
        let mut candidate = self.clone();
        candidate.credit(account, amount)?;
        candidate.total_supply = candidate.total_supply.checked_add(amount)?;
        let tx_id = TxId::from_serializable(
            "apex-genesis-credit-v1",
            &(account, amount, candidate.journal.len()),
        )?;
        candidate.journal.push(JournalEntry {
            tx_id,
            op: JournalOp::GenesisCredit { account, amount },
        });
        candidate.verify_conservation()?;
        *self = candidate;
        Ok(tx_id)
    }

    pub fn balance_of(&self, account: AccountId) -> ApexResult<Amount> {
        Ok(self.ensure_account(account)?.balance)
    }

    pub fn intent_nonce(&self, account: AccountId) -> ApexResult<u64> {
        Ok(self.ensure_account(account)?.next_intent_nonce)
    }

    pub fn settlement_nonce(&self, account: AccountId) -> ApexResult<u64> {
        Ok(self.ensure_account(account)?.next_settlement_nonce)
    }

    pub fn total_supply(&self) -> Amount {
        self.total_supply
    }

    pub fn state_digest(&self) -> ApexResult<Digest> {
        Digest::from_serializable(
            "apex-ledger-state-v1",
            &(
                self.network_id,
                self.asset,
                self.current_epoch,
                &self.accounts,
                &self.intents,
                &self.seen_transactions,
                self.total_supply,
            ),
        )
    }

    pub fn open_intent(&mut self, signed: &SignedIntent) -> ApexResult<TxId> {
        let mut candidate = self.clone();
        let tx_id = candidate.open_intent_inner(signed)?;
        candidate.verify_conservation()?;
        *self = candidate;
        Ok(tx_id)
    }

    pub fn settle_intent(&mut self, signed: &SignedSettlement) -> ApexResult<TxId> {
        let mut candidate = self.clone();
        let tx_id = candidate.settle_intent_inner(signed)?;
        candidate.verify_conservation()?;
        *self = candidate;
        Ok(tx_id)
    }

    fn open_intent_inner(&mut self, signed: &SignedIntent) -> ApexResult<TxId> {
        signed.verify()?;
        let terms = signed.terms;
        let route = signed.route;

        if terms.network_id != self.network_id {
            return Err(ApexError::Policy("wrong network".to_owned()));
        }

        if terms.asset != self.asset {
            return Err(ApexError::AssetMismatch {
                expected: self.asset,
                received: terms.asset,
            });
        }

        if terms.policy.valid_after_epoch > terms.policy.expires_at_epoch {
            return Err(ApexError::Policy(
                "invalid intent validity window".to_owned(),
            ));
        }

        if self.current_epoch < terms.policy.valid_after_epoch
            || self.current_epoch > terms.policy.expires_at_epoch
        {
            return Err(ApexError::Policy(
                "intent is outside its validity window".to_owned(),
            ));
        }

        if self.intents.contains_key(&terms.intent_id) {
            return Err(ApexError::IntentAlreadyExists(terms.intent_id));
        }

        self.ensure_account(terms.beneficiary)?;
        self.ensure_account(route.solver)?;
        self.ensure_account(route.fee_recipient)?;
        self.ensure_account(route.credit.provider)?;
        if let Some(rebate_recipient) = route.rebate_recipient {
            self.ensure_account(rebate_recipient)?;
        }

        route.validate(terms.policy, terms.amount)?;

        let payer = self.ensure_account(terms.payer)?;
        if payer.next_intent_nonce != terms.payer_nonce {
            return Err(ApexError::NonceMismatch {
                account: terms.payer,
                expected: payer.next_intent_nonce,
                received: terms.payer_nonce,
            });
        }

        if payer.balance < terms.amount {
            return Err(ApexError::InsufficientFunds {
                account: terms.payer,
                available: payer.balance,
                required: terms.amount,
            });
        }

        let tx_id = signed.tx_id()?;
        if self.seen_transactions.contains(&tx_id) {
            return Err(ApexError::DuplicateTransaction(tx_id));
        }

        let payer_state = self
            .accounts
            .get_mut(&terms.payer)
            .ok_or(ApexError::AccountNotFound(terms.payer))?;
        payer_state.balance = payer_state.balance.checked_sub(terms.amount)?;
        payer_state.next_intent_nonce = payer_state
            .next_intent_nonce
            .checked_add(1)
            .ok_or(ApexError::NonceOverflow)?;

        self.intents.insert(
            terms.intent_id,
            IntentState {
                id: terms.intent_id,
                terms,
                route,
                settled: false,
            },
        );
        self.seen_transactions.insert(tx_id);
        self.journal.push(JournalEntry {
            tx_id,
            op: JournalOp::IntentDebit {
                account: terms.payer,
                intent_id: terms.intent_id,
                amount: terms.amount,
            },
        });

        Ok(tx_id)
    }

    fn settle_intent_inner(&mut self, signed: &SignedSettlement) -> ApexResult<TxId> {
        signed.verify()?;

        if signed.request.network_id != self.network_id {
            return Err(ApexError::Policy("wrong network".to_owned()));
        }

        let intent = self
            .intents
            .get(&signed.request.intent_id)
            .ok_or(ApexError::IntentNotFound(signed.request.intent_id))?
            .clone();
        if intent.settled {
            return Err(ApexError::IntentSettled(intent.id));
        }

        if signed.signer.account != intent.terms.beneficiary {
            return Err(ApexError::UnauthorizedSettlementSigner {
                expected: intent.terms.beneficiary,
                received: signed.signer.account,
            });
        }

        let expected_route = intent.route.route_digest()?;
        if signed.request.observed_route_digest != expected_route {
            return Err(ApexError::RouteDigestMismatch {
                intent_id: intent.id,
                expected: expected_route,
                received: signed.request.observed_route_digest,
            });
        }

        let settlement_nonce = self
            .ensure_account(signed.signer.account)?
            .next_settlement_nonce;
        if settlement_nonce != signed.request.settlement_nonce {
            return Err(ApexError::NonceMismatch {
                account: signed.signer.account,
                expected: settlement_nonce,
                received: signed.request.settlement_nonce,
            });
        }

        let tx_id = signed.tx_id()?;
        if self.seen_transactions.contains(&tx_id) {
            return Err(ApexError::DuplicateTransaction(tx_id));
        }

        let route = intent.route;
        let route_charges = route.gross_charges()?;
        let beneficiary_amount = intent.terms.amount.checked_sub(route_charges)?;
        let rebate_recipient = route.rebate_recipient.unwrap_or(intent.terms.beneficiary);

        self.credit(intent.terms.beneficiary, beneficiary_amount)?;
        self.credit(route.fee_recipient, route.operator_fee)?;
        self.credit(rebate_recipient, route.rebate_amount)?;

        self.intents
            .get_mut(&intent.id)
            .ok_or(ApexError::IntentNotFound(intent.id))?
            .settled = true;
        self.accounts
            .get_mut(&signed.signer.account)
            .ok_or(ApexError::AccountNotFound(signed.signer.account))?
            .next_settlement_nonce = settlement_nonce
            .checked_add(1)
            .ok_or(ApexError::NonceOverflow)?;
        self.seen_transactions.insert(tx_id);
        self.journal.push(JournalEntry {
            tx_id,
            op: JournalOp::IntentSettlement {
                intent_id: intent.id,
                beneficiary: intent.terms.beneficiary,
                beneficiary_amount,
                fee_recipient: route.fee_recipient,
                fee_amount: route.operator_fee,
                rebate_recipient,
                rebate_amount: route.rebate_amount,
            },
        });

        Ok(tx_id)
    }

    fn credit(&mut self, account: AccountId, amount: Amount) -> ApexResult<()> {
        self.ensure_account(account)?;
        let current = self.balance_of(account)?;
        self.accounts
            .get_mut(&account)
            .ok_or(ApexError::AccountNotFound(account))?
            .balance = current.checked_add(amount)?;
        Ok(())
    }

    fn ensure_account(&self, account: AccountId) -> ApexResult<&AccountState> {
        self.accounts
            .get(&account)
            .ok_or(ApexError::AccountNotFound(account))
    }

    pub fn verify_conservation(&self) -> ApexResult<()> {
        let mut liquid = Amount::zero();
        for account in self.accounts.values() {
            liquid = liquid.checked_add(account.balance)?;
        }

        let mut locked = Amount::zero();
        for intent in self.intents.values() {
            if !intent.settled {
                locked = locked.checked_add(intent.terms.amount)?;
            }
        }

        let observed = liquid.checked_add(locked)?;
        if observed != self.total_supply {
            return Err(ApexError::Conservation {
                asset: self.asset,
                expected: self.total_supply,
                observed,
            });
        }

        Ok(())
    }
}
