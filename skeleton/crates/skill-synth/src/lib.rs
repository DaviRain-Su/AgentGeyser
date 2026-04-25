//! SkillSynthesizer — deterministic IDL → Skill translation for the AgentGeyser Spike.
//!
//! No LLM calls. For every instruction in the IDL we emit one `Skill` with a
//! JSON Schema 2020-12 describing its parameters. Unknown Anchor arg kinds fall
//! back to `{ "type": "object" }` with a `tracing::warn!` diagnostic.
//!
//! M2: widened to carry Anchor v0.30+ discriminator, account metas, and
//! Borsh-typed args (`IdlType`). New fields are serde-default so legacy
//! Spike IDLs and JSON payloads continue to deserialize unchanged.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub id: String,
    pub name: Option<String>,
}

/// Borsh-mappable primitive for Anchor instruction args. Lower-case kebab
/// serde representation. `Pubkey` also accepts the v0.30 `publicKey` alias.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum IdlType {
    U8,
    U16,
    U32,
    U64,
    I64,
    Bool,
    String,
    #[serde(alias = "publicKey", alias = "Pubkey")]
    Pubkey,
    Bytes,
}

/// Lazy PDA description; shape matches Anchor IDL's `pda` object.
pub type IdlPda = Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct IdlAccount {
    pub name: String,
    #[serde(default, alias = "isMut")]
    pub is_mut: bool,
    #[serde(default, alias = "isSigner")]
    pub is_signer: bool,
    #[serde(default)]
    pub pda: Option<IdlPda>,
}

/// Instruction arg. Deserializes from either `{ "kind": "u64" }` (legacy /
/// Spike) or `{ "type": "u64" }` (Anchor v0.30+); both populate `kind`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct IdlInstructionArg {
    pub name: String,
    #[serde(default, alias = "type", alias = "kind")]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct IdlInstruction {
    pub name: String,
    #[serde(default)]
    pub args: Vec<IdlInstructionArg>,
    #[serde(default)]
    pub accounts: Vec<IdlAccount>,
    #[serde(default)]
    pub discriminator: Option<[u8; 8]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Idl {
    pub version: String,
    pub name: String,
    pub instructions: Vec<IdlInstruction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillAccountSpec {
    pub name: String,
    pub is_mut: bool,
    pub is_signer: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillArgSpec {
    pub name: String,
    pub ty: IdlType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub skill_id: String,
    pub program_id: String,
    pub program_name: Option<String>,
    pub instruction_name: String,
    pub params_schema: Value,
    #[serde(default)]
    pub discriminator: [u8; 8],
    #[serde(default)]
    pub accounts: Vec<SkillAccountSpec>,
    #[serde(default)]
    pub args: Vec<SkillArgSpec>,
}

/// `sha256("global:<ix_name>")[..8]` — Anchor v<0.30 discriminator fallback.
pub fn compute_anchor_discriminator(ix_name: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"global:");
    h.update(ix_name.as_bytes());
    let d = h.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&d[..8]);
    out
}

fn parse_idl_type(s: &str) -> IdlType {
    serde_json::from_value(serde_json::Value::String(s.to_string())).unwrap_or(IdlType::Bytes)
}

/// Translate an Anchor arg kind to a JSON Schema 2020-12 fragment.
pub fn arg_schema(kind: &str) -> Value {
    match kind {
        "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" => {
            json!({ "type": "integer" })
        }
        "bool" => json!({ "type": "boolean" }),
        "string" | "String" => json!({ "type": "string" }),
        "publicKey" | "pubkey" | "Pubkey" => json!({
            "type": "string",
            "pattern": "^[1-9A-HJ-NP-Za-km-z]{32,44}$"
        }),
        "bytes" => json!({ "type": "string", "contentEncoding": "base64" }),
        other => {
            tracing::warn!(event = "unknown_arg_kind", kind = %other, "falling back to object schema");
            json!({ "type": "object" })
        }
    }
}

/// Build a JSON Schema describing the params object for an instruction.
pub fn instruction_schema(ix: &IdlInstruction) -> Value {
    let mut props = serde_json::Map::new();
    let mut required = Vec::with_capacity(ix.args.len());
    for arg in &ix.args {
        props.insert(arg.name.clone(), arg_schema(&arg.kind));
        required.push(Value::String(arg.name.clone()));
    }
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": ix.name,
        "type": "object",
        "properties": Value::Object(props),
        "required": required,
        "additionalProperties": false
    })
}

/// Deterministically synthesize one Skill per IDL instruction.
pub fn synthesize(program: &Program, idl: &Idl) -> Vec<Skill> {
    idl.instructions
        .iter()
        .map(|ix| Skill {
            skill_id: format!("{}::{}", program.id, ix.name),
            program_id: program.id.clone(),
            program_name: program.name.clone().or_else(|| Some(idl.name.clone())),
            instruction_name: ix.name.clone(),
            params_schema: instruction_schema(ix),
            discriminator: ix
                .discriminator
                .unwrap_or_else(|| compute_anchor_discriminator(&ix.name)),
            accounts: ix
                .accounts
                .iter()
                .map(|a| SkillAccountSpec {
                    name: a.name.clone(),
                    is_mut: a.is_mut,
                    is_signer: a.is_signer,
                })
                .collect(),
            args: ix
                .args
                .iter()
                .map(|a| SkillArgSpec {
                    name: a.name.clone(),
                    ty: parse_idl_type(&a.kind),
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonschema::JSONSchema;

    fn fixture() -> (Program, Idl) {
        let program = Program {
            id: "HELLO111111111111111111111111111111111111111".to_string(),
            name: Some("hello_world".to_string()),
        };
        let idl = Idl {
            version: "0.1.0".into(),
            name: "hello_world".into(),
            instructions: vec![
                IdlInstruction {
                    name: "initialize".into(),
                    args: vec![IdlInstructionArg {
                        name: "authority".into(),
                        kind: "publicKey".into(),
                    }],
                    ..Default::default()
                },
                IdlInstruction {
                    name: "greet".into(),
                    args: vec![IdlInstructionArg {
                        name: "name".into(),
                        kind: "string".into(),
                    }],
                    ..Default::default()
                },
                IdlInstruction {
                    name: "set_counter".into(),
                    args: vec![IdlInstructionArg {
                        name: "value".into(),
                        kind: "u64".into(),
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        (program, idl)
    }

    #[test]
    fn synthesize_emits_one_skill_per_instruction() {
        let (program, idl) = fixture();
        let skills = synthesize(&program, &idl);
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[0].skill_id, format!("{}::initialize", program.id));
        assert_eq!(skills[1].instruction_name, "greet");
        assert_eq!(skills[2].program_name.as_deref(), Some("hello_world"));
    }

    #[test]
    fn generated_schema_validates_valid_payload_and_rejects_invalid() {
        let (program, idl) = fixture();
        let skills = synthesize(&program, &idl);
        let validator = JSONSchema::compile(&skills[1].params_schema).expect("valid schema");
        assert!(validator.is_valid(&json!({ "name": "Alice" })));
        assert!(!validator.is_valid(&json!({ "name": 42 })));
        assert!(!validator.is_valid(&json!({})));
        let counter_validator =
            JSONSchema::compile(&skills[2].params_schema).expect("valid schema");
        assert!(counter_validator.is_valid(&json!({ "value": 7 })));
        assert!(!counter_validator.is_valid(&json!({ "value": "seven" })));
    }

    #[test]
    fn widened_idl_v030_roundtrip() {
        let raw = json!({
            "version": "0.1.0", "name": "hello_world",
            "address": "HELLO111111111111111111111111111111111111111",
            "instructions": [{
                "name": "greet",
                "discriminator": [1,2,3,4,5,6,7,8],
                "accounts": [
                    { "name": "user", "isMut": true, "isSigner": true },
                    { "name": "system_program", "isMut": false, "isSigner": false }
                ],
                "args": [{ "name": "name", "type": "string" }]
            }]
        });
        let idl: Idl = serde_json::from_value(raw).expect("parse v0.30 idl");
        assert_eq!(
            idl.address.as_deref(),
            Some("HELLO111111111111111111111111111111111111111")
        );
        let ix = &idl.instructions[0];
        assert_eq!(ix.discriminator, Some([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(ix.accounts.len(), 2);
        assert!(ix.accounts[0].is_mut && ix.accounts[0].is_signer);
        let program = Program {
            id: "HELLO111111111111111111111111111111111111111".into(),
            name: None,
        };
        let skill = &synthesize(&program, &idl)[0];
        assert_eq!(skill.discriminator, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(skill.accounts.len(), 2);
        assert_eq!(skill.accounts[0].name, "user");
        assert_eq!(skill.args[0].ty, IdlType::String);
    }

    #[test]
    fn legacy_idl_without_discriminator_falls_back() {
        let raw = json!({
            "version": "0.1.0", "name": "legacy",
            "instructions": [{ "name": "initialize",
                "args": [{ "name": "authority", "kind": "publicKey" }] }]
        });
        let idl: Idl = serde_json::from_value(raw).expect("parse legacy idl");
        assert!(idl.instructions[0].discriminator.is_none());
        assert!(idl.instructions[0].accounts.is_empty());
        let program = Program {
            id: "LEGACY1111111111111111111111111111111111111".into(),
            name: None,
        };
        let skill = &synthesize(&program, &idl)[0];
        assert_eq!(skill.discriminator, [175, 175, 109, 31, 13, 152, 155, 237]);
        assert_eq!(skill.args[0].ty, IdlType::Pubkey);
    }

    #[test]
    fn borsh_types_deserialize_all_variants() {
        let cases = [
            ("u8", IdlType::U8),
            ("u16", IdlType::U16),
            ("u32", IdlType::U32),
            ("u64", IdlType::U64),
            ("i64", IdlType::I64),
            ("bool", IdlType::Bool),
            ("string", IdlType::String),
            ("pubkey", IdlType::Pubkey),
            ("bytes", IdlType::Bytes),
        ];
        for (s, expected) in cases {
            let v: IdlType = serde_json::from_value(json!(s)).expect("parse IdlType");
            assert_eq!(v, expected, "kebab `{}`", s);
            let back = serde_json::to_value(v).unwrap();
            assert_eq!(back, json!(s));
        }
        let v: IdlType = serde_json::from_value(json!("publicKey")).expect("alias");
        assert_eq!(v, IdlType::Pubkey);
    }
}
