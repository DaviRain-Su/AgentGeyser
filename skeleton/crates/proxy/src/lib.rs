//! AgentGeyser proxy — HTTP JSON-RPC entry point.
//!
//! Methods: `ag_listSkills`, `ag_getIdl`, `ag_invokeSkill`.
//! `ag_invokeSkill` returns real (unsigned) Solana transaction bytes via
//! `tx-builder`. Non-custodial invariant: the proxy never signs anything.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::{extract::State, routing::post, Json, Router};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use idl_registry::{Idl, IdlInstruction, IdlInstructionArg, IdlRegistry};
use serde::Deserialize;
use serde_json::{json, Value};
use solana_sdk::{
    hash::Hash,
    instruction::AccountMeta,
    pubkey::Pubkey,
};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<IdlRegistry>,
    pub rpc_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcReq {
    #[serde(default)]
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// Build the axum `Router` backed by an `AppState`.
pub fn router(state: AppState) -> Router {
    Router::new().route("/", post(rpc_handler)).with_state(state)
}

async fn rpc_handler(State(st): State<AppState>, Json(req): Json<JsonRpcReq>) -> Json<Value> {
    let id = req.id.clone();
    match req.method.as_str() {
        "ag_listSkills" => Json(ok(id, serde_json::to_value(st.registry.list_skills()).unwrap_or(Value::Null))),
        "ag_getIdl" => {
            let program_id = req
                .params
                .get("program_id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match st.registry.get_idl(program_id) {
                Some(idl) => Json(ok(id, serde_json::to_value(idl).unwrap_or(Value::Null))),
                None => Json(err(id, -32004, "idl not found")),
            }
        }
        "ag_invokeSkill" => match handle_invoke(&st, &req.params).await {
            Ok(result) => Json(ok(id, result)),
            Err((code, msg)) => Json(err(id, code, &msg)),
        },
        _ => Json(err(id, -32601, "method not found")),
    }
}

async fn handle_invoke(st: &AppState, params: &Value) -> Result<Value, (i32, String)> {
    let skill_id = params
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if skill_id.is_empty() {
        return Err((-32602, "missing skill_id".into()));
    }
    let payer_str = params
        .get("payer")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing payer".into()))?;
    let payer = Pubkey::from_str(payer_str).map_err(|e| (-32602, format!("invalid payer: {e}")))?;

    let accounts_obj = params.get("accounts").cloned().unwrap_or(json!({}));
    let accounts_map = parse_named_pubkeys(&accounts_obj)?;
    let args = params.get("args").cloned().unwrap_or(json!({}));

    let blockhash = match &st.rpc_url {
        Some(url) => fetch_latest_blockhash(url).await
            .map_err(|e| (-32000, format!("getLatestBlockhash failed: {e}")))?,
        None => Hash::new_from_array([0u8; 32]),
    };

    let bytes = if skill_id.starts_with("spl-token::") {
        let skill = st
            .registry
            .skills
            .get(&skill_id)
            .map(|e| e.value().clone())
            .ok_or((-32004, "skill not found".into()))?;
        let program_id = Pubkey::from_str(&skill.program_id)
            .map_err(|e| (-32000, format!("invalid skill program_id: {e}")))?;
        let metas: Vec<AccountMeta> = skill
            .accounts
            .iter()
            .map(|a| {
                let pk = accounts_map
                    .get(&a.name)
                    .copied()
                    .ok_or((-32602, format!("missing account: {}", a.name)))?;
                Ok(if a.is_mut && a.is_signer {
                    AccountMeta::new(pk, true)
                } else if a.is_mut {
                    AccountMeta::new(pk, false)
                } else if a.is_signer {
                    AccountMeta::new_readonly(pk, true)
                } else {
                    AccountMeta::new_readonly(pk, false)
                })
            })
            .collect::<Result<_, (i32, String)>>()?;
        // skill.discriminator holds the 1-byte tag in slot 0 (padded with zeros);
        // pack tag + u64 LE amount for the SPL Transfer native path.
        let amount = args.get("amount").and_then(Value::as_u64)
            .ok_or((-32602, "missing u64 arg: amount".into()))?;
        let mut ix_data = vec![skill.discriminator[0]];
        ix_data.extend_from_slice(&amount.to_le_bytes());
        tx_builder::build_native_unsigned_tx(program_id, ix_data, metas, payer, blockhash)
            .map_err(|e| (-32000, format!("tx build failed: {e}")))?
    } else {
        if !st.registry.has_skill(&skill_id) {
            // Lazy IDL fetch on cache miss: for skill_id `<base58>::<ix_name>`,
            // attempt a single on-chain Anchor IDL fetch, then retry the lookup.
            if let Some((pid, _)) = skill_id.split_once("::") {
                if Pubkey::from_str(pid).is_ok() {
                    let _ = st.registry.try_fetch_and_register(pid).await;
                }
            }
            if !st.registry.has_skill(&skill_id) {
                return Err((-32004, "skill not found".into()));
            }
        }
        let skill = st.registry.skills.get(&skill_id).map(|e| e.value().clone()).unwrap();
        tx_builder::build_anchor_unsigned_tx(&skill, &args, &accounts_map, payer, blockhash)
            .map_err(|e| (-32000, format!("tx build failed: {e}")))?
    };

    Ok(json!({
        "skill_id": skill_id,
        "transaction_base64": B64.encode(bytes),
    }))
}

fn parse_named_pubkeys(v: &Value) -> Result<HashMap<String, Pubkey>, (i32, String)> {
    let obj = v.as_object().ok_or((-32602, "accounts must be object".into()))?;
    obj.iter()
        .map(|(k, val)| {
            let s = val.as_str().ok_or((-32602, format!("account `{k}` must be string")))?;
            let pk = Pubkey::from_str(s).map_err(|e| (-32602, format!("account `{k}` invalid: {e}")))?;
            Ok((k.clone(), pk))
        })
        .collect()
}

async fn fetch_latest_blockhash(rpc_url: &str) -> anyhow::Result<Hash> {
    let body = json!({
        "jsonrpc": "2.0", "id": 1, "method": "getLatestBlockhash", "params": []
    });
    let resp: Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let s = resp
        .pointer("/result/value/blockhash")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing result.value.blockhash"))?;
    Hash::from_str(s).map_err(|e| anyhow::anyhow!("blockhash parse: {e}"))
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// A small demo IDL used by the default binary so a fresh boot has something to list.
pub fn sample_hello_idl() -> Idl {
    Idl {
        version: "0.1.0".into(),
        name: "hello_world".into(),
        instructions: vec![
            IdlInstruction {
                name: "initialize".into(),
                args: vec![IdlInstructionArg { name: "authority".into(), kind: "publicKey".into(), ..Default::default() }],
                ..Default::default()
            },
            IdlInstruction {
                name: "greet".into(),
                args: vec![IdlInstructionArg { name: "name".into(), kind: "string".into(), ..Default::default() }],
                ..Default::default()
            },
            IdlInstruction {
                name: "set_counter".into(),
                args: vec![IdlInstructionArg { name: "value".into(), kind: "u64".into(), ..Default::default() }],
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

pub const DEMO_PROGRAM_ID: &str = "HELLO111111111111111111111111111111111111111";
