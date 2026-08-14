use serde::Serialize;

use crate::{ApexError, ApexLedger, ApexResult, Digest, VERSION};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StateCheckpoint {
    pub sequence: u64,
    pub epoch: u64,
    pub state_digest: Digest,
    pub journal_digest: Digest,
    pub previous_digest: Option<Digest>,
    pub checkpoint_digest: Digest,
    pub version: &'static str,
}

impl StateCheckpoint {
    pub fn build(
        ledger: &ApexLedger,
        sequence: u64,
        previous_digest: Option<Digest>,
    ) -> ApexResult<Self> {
        if sequence == 0 {
            return Err(ApexError::InvalidConfiguration(
                "checkpoint sequence must be positive".to_owned(),
            ));
        }
        let state_digest = ledger.state_digest()?;
        let journal_digest =
            Digest::from_serializable("apex-checkpoint-journal-v1", &ledger.journal())?;
        let checkpoint_digest = Digest::from_serializable(
            "apex-state-checkpoint-v1",
            &(
                sequence,
                ledger.current_epoch(),
                state_digest,
                journal_digest,
                previous_digest,
                VERSION,
            ),
        )?;
        Ok(Self {
            sequence,
            epoch: ledger.current_epoch(),
            state_digest,
            journal_digest,
            previous_digest,
            checkpoint_digest,
            version: VERSION,
        })
    }

    pub fn verify(self, ledger: &ApexLedger) -> ApexResult<()> {
        let rebuilt = Self::build(ledger, self.sequence, self.previous_digest)?;
        if rebuilt != self {
            return Err(ApexError::State(
                "checkpoint does not match ledger state".to_owned(),
            ));
        }
        Ok(())
    }
}
