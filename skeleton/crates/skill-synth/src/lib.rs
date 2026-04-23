//! SkillSynthesizer — deterministic IDL → Skill translation for the AgentGeyser Spike.
//!
//! No LLM calls. For every instruction in the IDL we emit one `Skill` with a
//! JSON Schema 2020-12 describing its parameters. Unknown Anchor arg kinds fall
//! back to `{ "type": "object" }` with a `tracing::warn!` diagnostic.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Program {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IdlInstructionArg {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IdlInstruction {
    pub name: String,
    pub args: Vec<IdlInstructionArg>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Idl {
    pub version: String,
    pub name: String,
    pub instructions: Vec<IdlInstruction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub skill_id: String,
    pub program_id: String,
    pub program_name: Option<String>,
    pub instruction_name: String,
    pub params_schema: Value,
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
                    args: vec![IdlInstructionArg { name: "authority".into(), kind: "publicKey".into() }],
                },
                IdlInstruction {
                    name: "greet".into(),
                    args: vec![IdlInstructionArg { name: "name".into(), kind: "string".into() }],
                },
                IdlInstruction {
                    name: "set_counter".into(),
                    args: vec![IdlInstructionArg { name: "value".into(), kind: "u64".into() }],
                },
            ],
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

        // greet expects { name: string }
        let greet_schema = &skills[1].params_schema;
        let validator = JSONSchema::compile(greet_schema).expect("valid schema");
        assert!(validator.is_valid(&json!({ "name": "Alice" })));
        assert!(!validator.is_valid(&json!({ "name": 42 })));
        assert!(!validator.is_valid(&json!({})));

        // set_counter expects { value: integer }
        let counter_schema = &skills[2].params_schema;
        let counter_validator = JSONSchema::compile(counter_schema).expect("valid schema");
        assert!(counter_validator.is_valid(&json!({ "value": 7 })));
        assert!(!counter_validator.is_valid(&json!({ "value": "seven" })));
    }
}
