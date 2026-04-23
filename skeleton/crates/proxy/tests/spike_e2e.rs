//! End-to-end spike test: stream event → registry → HTTP → SDK contract.

use std::sync::Arc;
use std::time::Duration;

use idl_registry::{Idl, IdlInstruction, IdlInstructionArg, IdlRegistry, MockYellowstoneStream, YellowstoneEvent};
use proxy::{router, AppState, UNSIGNED_TX_PLACEHOLDER};
use serde_json::{json, Value};

fn fixture_idl() -> Idl {
    Idl {
        version: "0.1.0".into(),
        name: "hello_world".into(),
        instructions: vec![
            IdlInstruction { name: "greet".into(), args: vec![IdlInstructionArg { name: "name".into(), kind: "string".into(), ..Default::default() }], ..Default::default() },
            IdlInstruction { name: "set_counter".into(), args: vec![IdlInstructionArg { name: "value".into(), kind: "u64".into(), ..Default::default() }], ..Default::default() },
        ],
        ..Default::default()
    }
}

#[tokio::test]
async fn end_to_end_spike_flow() {
    // 1. Build registry with a mock IDL and feed a deployment event.
    let registry = Arc::new(IdlRegistry::new());
    let program_id = "HELLO111111111111111111111111111111111111111";
    registry.insert_mock_idl(program_id, fixture_idl());

    let stream = MockYellowstoneStream::new(vec![YellowstoneEvent::ProgramDeployed {
        program_id: program_id.to_string(),
    }]);
    registry.attach_stream(stream).await.unwrap();

    // 2. Launch the axum proxy on an ephemeral port.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(AppState { registry: Arc::clone(&registry) });
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/", addr);

    // 3. Poll ag_listSkills until skills appear (already populated but retry a few times just in case).
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
    assert!(skills.len() >= 2, "expected >=2 skills, got {}", skills.len());

    // 4. Call ag_invokeSkill with the first skill id.
    let skill_id = skills[0].get("skill_id").and_then(Value::as_str).unwrap().to_string();
    let resp: Value = client
        .post(&url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ag_invokeSkill",
            "params": { "skill_id": skill_id }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["result"]["skill_id"], Value::String(skill_id.clone()));
    assert_eq!(
        resp["result"]["transaction_base64"],
        Value::String(UNSIGNED_TX_PLACEHOLDER.to_string())
    );

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
            "params": { "skill_id": "does::not::exist" }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["error"]["code"], Value::from(-32004));
}
