//! tx-builder — pure Solana unsigned-transaction byte synthesis.
//!
//! No I/O, no async runtime, no key material. Callers are responsible for
//! fetching the recent blockhash, resolving account names to pubkeys, and
//! broadcasting the returned (still unsigned) transaction.
//!
//! Two entry points:
//!   * `build_anchor_unsigned_tx` — Anchor-style: 8-byte discriminator +
//!     Borsh-encoded args in IDL declaration order.
//!   * `build_native_unsigned_tx` — raw native-program path (e.g. SPL-Token):
//!     caller supplies fully-formed `ix_data`.
//!
//! Output is `bincode::serialize(&Transaction)` bytes ready for an external
//! wallet to attach the payer signature.

pub mod devnet_gate;
pub mod spl_token;

pub use spl_token::{
    build_spl_token_transfer, SplTokenTransferArgs, TxBuilderError, UnsignedTx,
    TOKEN_2022_PROGRAM_ID,
};

use anyhow::{anyhow, Context, Result};
use borsh::BorshSerialize;
use serde_json::Value;
use skill_synth::{IdlType, Skill, SkillArgSpec};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
};
use std::collections::HashMap;
use std::str::FromStr;

/// Build an unsigned Anchor instruction transaction.
pub fn build_anchor_unsigned_tx(
    skill: &Skill,
    args_json: &Value,
    named_accounts: &HashMap<String, Pubkey>,
    payer: Pubkey,
    blockhash: Hash,
) -> Result<Vec<u8>> {
    let metas: Vec<AccountMeta> = skill
        .accounts
        .iter()
        .map(|a| {
            let pk = named_accounts
                .get(&a.name)
                .ok_or_else(|| anyhow!("missing account: {}", a.name))?;
            Ok(meta_from_flags(*pk, a.is_mut, a.is_signer))
        })
        .collect::<Result<_>>()?;

    let mut data = Vec::with_capacity(8 + skill.args.len() * 8);
    data.extend_from_slice(&skill.discriminator);
    for arg in &skill.args {
        let v = args_json
            .get(&arg.name)
            .ok_or_else(|| anyhow!("missing arg: {}", arg.name))?;
        encode_borsh_arg(&mut data, arg, v)?;
    }

    let program_id = Pubkey::from_str(&skill.program_id)
        .map_err(|e| anyhow!("invalid program_id `{}`: {}", skill.program_id, e))?;
    let ix = Instruction {
        program_id,
        accounts: metas,
        data,
    };
    let msg = Message::new_with_blockhash(&[ix], Some(&payer), &blockhash);
    let tx = Transaction::new_unsigned(msg);
    bincode::serialize(&tx).context("bincode serialize transaction")
}

/// Build an unsigned raw-native-program transaction (SPL-Token etc.).
pub fn build_native_unsigned_tx(
    program_id: Pubkey,
    ix_data: Vec<u8>,
    account_metas: Vec<AccountMeta>,
    payer: Pubkey,
    blockhash: Hash,
) -> Result<Vec<u8>> {
    let ix = Instruction {
        program_id,
        accounts: account_metas,
        data: ix_data,
    };
    let msg = Message::new_with_blockhash(&[ix], Some(&payer), &blockhash);
    let tx = Transaction::new_unsigned(msg);
    bincode::serialize(&tx).context("bincode serialize transaction")
}

fn meta_from_flags(pk: Pubkey, is_mut: bool, is_sig: bool) -> AccountMeta {
    match (is_mut, is_sig) {
        (true, true) => AccountMeta::new(pk, true),
        (true, false) => AccountMeta::new(pk, false),
        (false, true) => AccountMeta::new_readonly(pk, true),
        (false, false) => AccountMeta::new_readonly(pk, false),
    }
}

fn encode_borsh_arg(out: &mut Vec<u8>, arg: &SkillArgSpec, v: &Value) -> Result<()> {
    match arg.ty {
        IdlType::U8 => {
            let n: u8 = as_u64(v, &arg.name)?
                .try_into()
                .map_err(|_| anyhow!("arg `{}` overflows u8", arg.name))?;
            BorshSerialize::serialize(&n, out)?;
        }
        IdlType::U16 => {
            let n: u16 = as_u64(v, &arg.name)?
                .try_into()
                .map_err(|_| anyhow!("arg `{}` overflows u16", arg.name))?;
            BorshSerialize::serialize(&n, out)?;
        }
        IdlType::U32 => {
            let n: u32 = as_u64(v, &arg.name)?
                .try_into()
                .map_err(|_| anyhow!("arg `{}` overflows u32", arg.name))?;
            BorshSerialize::serialize(&n, out)?;
        }
        IdlType::U64 => BorshSerialize::serialize(&as_u64(v, &arg.name)?, out)?,
        IdlType::I64 => {
            let n = v
                .as_i64()
                .ok_or_else(|| anyhow!("arg `{}` expected i64", arg.name))?;
            BorshSerialize::serialize(&n, out)?;
        }
        IdlType::Bool => {
            let b = v
                .as_bool()
                .ok_or_else(|| anyhow!("arg `{}` expected bool", arg.name))?;
            BorshSerialize::serialize(&b, out)?;
        }
        IdlType::String => {
            let s = v
                .as_str()
                .ok_or_else(|| anyhow!("arg `{}` expected string", arg.name))?;
            BorshSerialize::serialize(&s.to_string(), out)?;
        }
        IdlType::Pubkey => {
            let s = v
                .as_str()
                .ok_or_else(|| anyhow!("arg `{}` expected base58 pubkey", arg.name))?;
            let pk = Pubkey::from_str(s)
                .map_err(|e| anyhow!("arg `{}` invalid pubkey: {}", arg.name, e))?;
            out.extend_from_slice(&pk.to_bytes());
        }
        IdlType::Bytes => {
            // Encoded as Borsh `Vec<u8>` (u32 LE length + bytes). For Bytes
            // args we accept a JSON array of u8 to avoid bringing in a base64
            // dep just for tests.
            let arr = v
                .as_array()
                .ok_or_else(|| anyhow!("arg `{}` expected u8 array", arg.name))?;
            let raw: Vec<u8> = arr
                .iter()
                .map(|x| x.as_u64().and_then(|n| u8::try_from(n).ok()))
                .collect::<Option<_>>()
                .ok_or_else(|| anyhow!("arg `{}` invalid u8 array", arg.name))?;
            BorshSerialize::serialize(&raw, out)?;
        }
    }
    Ok(())
}

fn as_u64(v: &Value, name: &str) -> Result<u64> {
    v.as_u64()
        .ok_or_else(|| anyhow!("arg `{}` expected unsigned integer", name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use skill_synth::SkillAccountSpec;

    fn pk(b: u8) -> Pubkey {
        Pubkey::new_from_array([b; 32])
    }

    fn greet_skill() -> Skill {
        let mut h = Sha256::new();
        h.update(b"global:greet");
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&h.finalize()[..8]);
        Skill {
            skill_id: "HELLO::greet".into(),
            program_id: pk(7).to_string(),
            program_name: Some("hello_world".into()),
            instruction_name: "greet".into(),
            params_schema: serde_json::json!({}),
            discriminator: disc,
            accounts: vec![SkillAccountSpec {
                name: "user".into(),
                is_mut: true,
                is_signer: true,
            }],
            args: vec![SkillArgSpec {
                name: "name".into(),
                ty: IdlType::String,
            }],
        }
    }

    #[test]
    fn anchor_hello_world_greet_world() {
        let skill = greet_skill();
        let mut named = HashMap::new();
        named.insert("user".to_string(), pk(9));
        let bytes = build_anchor_unsigned_tx(
            &skill,
            &serde_json::json!({ "name": "world" }),
            &named,
            pk(1),
            Hash::new_from_array([0u8; 32]),
        )
        .expect("build ok");

        let tx: Transaction = bincode::deserialize(&bytes).expect("deserialize");
        let ix = &tx.message.instructions[0];
        let mut h = Sha256::new();
        h.update(b"global:greet");
        let expected: [u8; 8] = h.finalize()[..8].try_into().unwrap();
        assert_eq!(&ix.data[..8], &expected);
        assert_eq!(&ix.data[8..12], &5u32.to_le_bytes());
        assert_eq!(&ix.data[12..17], b"world");
        assert_eq!(ix.data.len(), 17);
    }

    #[test]
    fn spl_token_transfer_amount_1000() {
        let mut ix_data = vec![3u8];
        ix_data.extend_from_slice(&1000u64.to_le_bytes());
        let metas = vec![
            AccountMeta::new(pk(10), false),
            AccountMeta::new(pk(11), false),
            AccountMeta::new_readonly(pk(12), true),
        ];
        let bytes = build_native_unsigned_tx(
            pk(42),
            ix_data,
            metas,
            pk(12),
            Hash::new_from_array([0u8; 32]),
        )
        .expect("build ok");
        let tx: Transaction = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(
            tx.message.instructions[0].data,
            vec![3u8, 0xE8, 0x03, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn missing_account_errors() {
        let skill = greet_skill();
        let named: HashMap<String, Pubkey> = HashMap::new();
        let err = build_anchor_unsigned_tx(
            &skill,
            &serde_json::json!({ "name": "x" }),
            &named,
            pk(1),
            Hash::new_from_array([0u8; 32]),
        )
        .expect_err("should fail");
        assert!(format!("{err}").contains("missing account"));
    }
}
