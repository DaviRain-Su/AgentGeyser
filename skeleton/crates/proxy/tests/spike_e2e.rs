//! End-to-end spike test: stream event → registry → HTTP → SDK contract.

use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use idl_registry::{
    Idl, IdlInstruction, IdlInstructionArg, IdlRegistry, MockYellowstoneStream, YellowstoneEvent,
};
use proxy::{router, AppState};
use serde_json::{json, Value};
use solana_sdk::transaction::Transaction;

fn fixture_idl() -> Idl {
    Idl {
        version: "0.1.0".into(),
        name: "hello_world".into(),
        instructions: vec![
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
    }
}

#[tokio::test]
async fn end_to_end_spike_flow() {
    // 1. Build registry with a mock IDL and feed a deployment event.
    let registry = Arc::new(IdlRegistry::new());
    // Valid base58 32-byte pubkey (all-ones decodes to all-zeros = System Program addr).
    let program_id = "11111111111111111111111111111111";
    registry.insert_mock_idl(program_id, fixture_idl());

    let stream = MockYellowstoneStream::new(vec![YellowstoneEvent::ProgramDeployed {
        program_id: program_id.to_string(),
    }]);
    registry.attach_stream(stream).await.unwrap();

    // 2. Launch the axum proxy on an ephemeral port (mock mode → deterministic fake blockhash).
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(AppState {
        registry: Arc::clone(&registry),
        rpc_url: None,
    });
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/", addr);

    // 3. Poll ag_listSkills until skills appear.
    let mut skills: Vec<Value> = Vec::new();
    for _ in 0..20 {
        let resp: Value = client
            .post(&url)
            .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "ag_listSkills" }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if let Some(arr) = resp.get("result").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                skills = arr.clone();
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        skills.len() >= 2,
        "expected >=2 skills, got {}",
        skills.len()
    );

    // 4. Invoke `set_counter` (deterministic u64 arg, no accounts).
    let target_id = format!("{}::set_counter", program_id);
    // Distinct valid 32-byte base58 payer pubkey (SysvarC1ock... all printable base58).
    let payer = "SysvarC1ock11111111111111111111111111111111";
    let resp: Value = client
        .post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ag_invokeSkill",
            "params": {
                "skill_id": target_id,
                "args": { "value": 7 },
                "accounts": {},
                "payer": payer
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["result"]["skill_id"], Value::String(target_id.clone()));
    let tx_b64 = resp["result"]["transaction_base64"]
        .as_str()
        .expect("base64 string");
    let raw = B64.decode(tx_b64).expect("base64 decode");
    let tx: Transaction = bincode::deserialize(&raw).expect("bincode decode Transaction");
    assert_eq!(tx.message.instructions.len(), 1);
    let ix = &tx.message.instructions[0];
    // program_id_index must resolve to the skill's program (not the payer at [0]).
    let resolved = &tx.message.account_keys[ix.program_id_index as usize];
    assert_eq!(resolved.to_string(), program_id);
    // Instruction data begins with the 8-byte Anchor discriminator, then Borsh u64 LE = 7.
    assert_eq!(ix.data.len(), 8 + 8);
    assert_eq!(&ix.data[8..], &7u64.to_le_bytes());

    // 5. ag_getIdl returns the stored IDL.
    let resp: Value = client
        .post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "ag_getIdl",
            "params": { "program_id": program_id }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["result"]["name"], Value::String("hello_world".into()));

    // 6. Unknown skill surface.
    let resp: Value = client
        .post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "ag_invokeSkill",
            "params": { "skill_id": "does::not::exist", "args": {}, "accounts": {}, "payer": payer }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["error"]["code"], Value::from(-32004));
}
