//! SPL Token 2022 unsigned-transfer builder.
//!
//! Builds an UNSIGNED `VersionedTransaction` (MessageV0) carrying a single
//! SPL-Token-2022 `Transfer` instruction. Non-custodial: no key material is
//! loaded, no signing is performed — the returned `UnsignedTx` is base64
//! bytes ready for client-side signing.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::{v0::Message as MessageV0, VersionedMessage},
    pubkey,
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use thiserror::Error;

/// SPL Token 2022 program id (the ONE base58 literal permitted in this crate).
pub const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// `transfer` opcode in the SPL-Token / Token-2022 instruction enum.
const SPL_TOKEN_TRANSFER_OPCODE: u8 = 3;

#[derive(Debug, Error)]
pub enum TxBuilderError {
    #[error("legacy SPL-Token program not supported by this builder")]
    LegacyNotSupported,
    #[error("bincode serialize: {0}")]
    Serialize(#[from] bincode::Error),
    #[error("message compile: {0}")]
    MessageCompile(String),
}

#[derive(Debug, Clone)]
pub struct SplTokenTransferArgs {
    pub source_ata: Pubkey,
    pub destination_ata: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub mint: Pubkey,
    pub recent_blockhash: Hash,
    /// If `true`, target legacy SPL-Token; currently returns
    /// `LegacyNotSupported` (keeps the crate VX.4-compliant: one base58 literal).
    pub legacy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTx {
    /// base64( bincode(VersionedTransaction) ) with an empty signature slot.
    pub tx_base64: String,
    /// base64( bincode(MessageV0) ).
    pub message_base64: String,
    /// Base58 echo of `recent_blockhash`.
    pub recent_blockhash: String,
}

/// Build an UNSIGNED Token-2022 transfer transaction.
pub fn build_spl_token_transfer(args: SplTokenTransferArgs) -> Result<UnsignedTx, TxBuilderError> {
    if args.legacy {
        return Err(TxBuilderError::LegacyNotSupported);
    }
    let program_id = TOKEN_2022_PROGRAM_ID;
    // Mint is not referenced by the non-checked `Transfer` ix, but we bind it
    // into `_` to keep the struct ergonomic for future `transfer_checked`.
    let _ = args.mint;

    let mut data = Vec::with_capacity(9);
    data.push(SPL_TOKEN_TRANSFER_OPCODE);
    data.extend_from_slice(&args.amount.to_le_bytes());

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(args.source_ata, false),
            AccountMeta::new(args.destination_ata, false),
            AccountMeta::new_readonly(args.owner, true),
        ],
        data,
    };

    let msg = MessageV0::try_compile(&args.owner, &[ix], &[], args.recent_blockhash)
        .map_err(|e| TxBuilderError::MessageCompile(format!("{e:?}")))?;

    let versioned_tx = VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(msg.clone()),
    };

    Ok(UnsignedTx {
        tx_base64: B64.encode(bincode::serialize(&versioned_tx)?),
        message_base64: B64.encode(bincode::serialize(&msg)?),
        recent_blockhash: args.recent_blockhash.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::message::v0::Message as MessageV0;

    fn sample_args(amount: u64, legacy: bool) -> SplTokenTransferArgs {
        SplTokenTransferArgs {
            source_ata: Pubkey::new_unique(),
            destination_ata: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount,
            mint: Pubkey::new_unique(),
            recent_blockhash: Hash::new_unique(),
            legacy,
        }
    }

    #[test]
    fn token_2022_transfer_happy_path() {
        let args = sample_args(1_000, false);
        let owner = args.owner;
        let out = build_spl_token_transfer(args).expect("builds");

        // Decode MessageV0 and assert structural invariants.
        let raw = B64.decode(&out.message_base64).expect("b64 decode msg");
        let msg: MessageV0 = bincode::deserialize(&raw).expect("deser MessageV0");

        assert_eq!(msg.instructions.len(), 1, "exactly one instruction");
        let ix = &msg.instructions[0];
        let program_id = msg.account_keys[ix.program_id_index as usize];
        assert_eq!(
            program_id, TOKEN_2022_PROGRAM_ID,
            "program id is Token-2022"
        );
        assert_eq!(
            ix.data[0], SPL_TOKEN_TRANSFER_OPCODE,
            "first byte is SPL transfer opcode (0x03)"
        );
        assert_eq!(
            msg.header.num_required_signatures, 1,
            "exactly one required signature (owner)"
        );
        assert_eq!(msg.account_keys[0], owner, "fee payer is owner");

        // Versioned tx round-trips with a single unsigned signature slot.
        let tx_raw = B64.decode(&out.tx_base64).expect("b64 decode tx");
        let vtx: VersionedTransaction = bincode::deserialize(&tx_raw).expect("deser vtx");
        assert_eq!(vtx.signatures.len(), 1);
        assert_eq!(
            vtx.signatures[0],
            Signature::default(),
            "signature is zeroed"
        );
    }

    #[test]
    fn legacy_branch_returns_error() {
        let args = sample_args(42, true);
        let err = build_spl_token_transfer(args).expect_err("legacy unsupported");
        assert!(matches!(err, TxBuilderError::LegacyNotSupported));
    }

    #[test]
    fn zero_amount_still_builds_valid_ix() {
        let args = sample_args(0, false);
        let out = build_spl_token_transfer(args).expect("builds");
        let raw = B64.decode(&out.message_base64).expect("b64 decode msg");
        let msg: MessageV0 = bincode::deserialize(&raw).expect("deser MessageV0");
        assert_eq!(msg.instructions.len(), 1);
        let ix = &msg.instructions[0];
        assert_eq!(ix.data[0], SPL_TOKEN_TRANSFER_OPCODE);
        assert_eq!(&ix.data[1..9], &0u64.to_le_bytes(), "amount is zero LE");
    }
}
