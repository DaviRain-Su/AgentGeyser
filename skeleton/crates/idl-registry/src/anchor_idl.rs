//! On-chain Anchor IDL fetcher.
//!
//! `fetch_anchor_idl` issues JSON-RPC `getAccountInfo` (base64) and decodes
//! the standard Anchor IDL account layout: [8B discriminator][32B authority]
//! [4B u32-le payload-len][zlib(JSON IDL)]. The pure helper
//! `decode_anchor_idl_payload` is tested offline.
//!
//! `fetch_anchor_idl` derives the canonical Anchor IDL PDA before fetching:
//! first the v0.30+ `[b"anchor:idl"]` PDA, then the legacy seeded address.

use std::io::Read;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use flate2::read::ZlibDecoder;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;

pub use skill_synth::{
    compute_anchor_discriminator, Idl, IdlAccount, IdlInstruction, IdlInstructionArg, IdlPda,
    IdlType,
};

const DISCRIMINATOR_LEN: usize = 8;
const AUTHORITY_LEN: usize = 32;
const LEN_PREFIX_LEN: usize = 4;
const HEADER_LEN: usize = DISCRIMINATOR_LEN + AUTHORITY_LEN + LEN_PREFIX_LEN;

/// Validate that `s` is a 32-byte base58 Solana address; return canonical form.
pub fn validate_program_id(s: &str) -> Result<String> {
    let raw = bs58::decode(s)
        .into_vec()
        .context("program_id is not valid base58")?;
    if raw.len() != 32 {
        return Err(anyhow!(
            "program_id must decode to 32 bytes, got {}",
            raw.len()
        ));
    }
    Ok(bs58::encode(raw).into_string())
}

/// Anchor v0.30+ IDL PDA: single `anchor:idl` seed under the program id.
pub fn anchor_idl_pda_primary(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"anchor:idl"], program_id).0
}

/// Legacy Anchor IDL address: seeded from the empty-seed program authority PDA.
pub fn anchor_idl_pda_legacy(program_id: &Pubkey) -> Result<Pubkey> {
    let base = Pubkey::find_program_address(&[], program_id).0;
    Pubkey::create_with_seed(&base, "anchor:idl", program_id)
        .context("derive legacy anchor idl pda failed")
}

/// Decode a raw Anchor IDL account payload into an `Idl`. Pure (no I/O).
///
/// For every instruction whose JSON did not provide a v0.30+ `discriminator`
/// field, the missing slot is filled in with `compute_anchor_discriminator`
/// so downstream consumers can treat the `Idl` as fully populated.
pub fn decode_anchor_idl_payload(bytes: &[u8]) -> Result<Idl> {
    if bytes.len() < HEADER_LEN {
        return Err(anyhow!(
            "anchor idl account too short: {} bytes",
            bytes.len()
        ));
    }
    let len_bytes: [u8; 4] = bytes[DISCRIMINATOR_LEN + AUTHORITY_LEN..HEADER_LEN]
        .try_into()
        .map_err(|_| anyhow!("failed to read length prefix"))?;
    let n = u32::from_le_bytes(len_bytes) as usize;
    let compressed = bytes
        .get(HEADER_LEN..HEADER_LEN + n)
        .ok_or_else(|| anyhow!("compressed payload truncated (want {} bytes)", n))?;
    let mut json = Vec::new();
    ZlibDecoder::new(compressed)
        .read_to_end(&mut json)
        .context("zlib inflate failed")?;
    let mut idl: Idl = serde_json::from_slice(&json).context("anchor idl json parse failed")?;
    for ix in idl.instructions.iter_mut() {
        if ix.discriminator.is_none() {
            ix.discriminator = Some(compute_anchor_discriminator(&ix.name));
        }
    }
    Ok(idl)
}

async fn fetch_anchor_idl_at(
    client: &reqwest::Client,
    rpc_url: &str,
    address: &Pubkey,
) -> Result<Option<Idl>> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [address.to_string(), { "encoding": "base64" }],
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("getAccountInfo request failed")?
        .error_for_status()
        .context("getAccountInfo returned HTTP error")?
        .json()
        .await
        .context("getAccountInfo response was not valid JSON")?;
    let value = match resp.pointer("/result/value") {
        None => return Ok(None),
        Some(v) if v.is_null() => return Ok(None),
        Some(v) => v,
    };
    let encoded = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing result.value.data[0] string"))?;
    let raw = B64.decode(encoded).context("base64 decode failed")?;
    match decode_anchor_idl_payload(&raw) {
        Ok(idl) => Ok(Some(idl)),
        Err(err) => {
            tracing::debug!(address = %address, error = %err, "anchor idl candidate failed to decode");
            Ok(None)
        }
    }
}

/// Fetch the Anchor IDL via JSON-RPC `getAccountInfo` and decode it.
/// Returns `Ok(None)` when both the primary and legacy IDL PDAs are absent or
/// do not contain a valid Anchor IDL account layout.
pub async fn fetch_anchor_idl(rpc_url: &str, program_id: &str) -> Result<Option<Idl>> {
    let address = validate_program_id(program_id)?;
    let program_id =
        Pubkey::from_str(&address).context("validated program_id failed Pubkey parse")?;
    let client = reqwest::Client::new();

    let primary = anchor_idl_pda_primary(&program_id);
    if let Some(idl) = fetch_anchor_idl_at(&client, rpc_url, &primary).await? {
        return Ok(Some(idl));
    }

    let legacy = anchor_idl_pda_legacy(&program_id)?;
    fetch_anchor_idl_at(&client, rpc_url, &legacy).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use solana_sdk::pubkey::Pubkey;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Write;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn build_fixture_bytes() -> Vec<u8> {
        let idl_json = serde_json::json!({
            "version": "0.1.0",
            "name": "hello_world",
            "instructions": [
                { "name": "initialize", "args": [] },
                { "name": "greet", "args": [{ "name": "name", "kind": "string" }] },
                { "name": "set_counter", "args": [{ "name": "value", "kind": "u64" }] }
            ]
        });
        let json_bytes = serde_json::to_vec(&idl_json).expect("serialize");
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&json_bytes).expect("zlib write");
        let compressed = enc.finish().expect("zlib finish");

        let mut buf = Vec::with_capacity(HEADER_LEN + compressed.len());
        buf.extend_from_slice(&[0u8; DISCRIMINATOR_LEN]);
        buf.extend_from_slice(&[1u8; AUTHORITY_LEN]);
        buf.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        buf.extend_from_slice(&compressed);
        buf
    }

    #[test]
    fn decode_anchor_idl_fixture() {
        let idl = decode_anchor_idl_payload(&build_fixture_bytes()).expect("decode");
        assert_eq!(idl.name, "hello_world");
        assert_eq!(idl.instructions.len(), 3);
        assert_eq!(idl.instructions[1].name, "greet");
    }

    #[test]
    fn decode_rejects_short_buffer() {
        let err = decode_anchor_idl_payload(&[0u8; 10]).unwrap_err();
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn validate_program_id_requires_32_bytes() {
        let err = validate_program_id("abc").unwrap_err();
        assert!(err.to_string().contains("32 bytes"));
    }

    async fn serve_accounts(
        accounts: HashMap<String, Vec<u8>>,
        max_requests: usize,
        requests: Arc<Mutex<Vec<String>>>,
    ) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            for _ in 0..max_requests {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                let req = String::from_utf8_lossy(&buf[..n]);
                let body = req.split("\r\n\r\n").nth(1).unwrap_or_default();
                let body: Value = serde_json::from_str(body).unwrap();
                let address = body
                    .pointer("/params/0")
                    .and_then(Value::as_str)
                    .unwrap()
                    .to_string();
                requests.lock().unwrap().push(address.clone());
                let value = accounts.get(&address).map(|bytes| {
                    serde_json::json!({
                        "data": [B64.encode(bytes), "base64"],
                        "executable": false,
                        "lamports": 0,
                        "owner": "11111111111111111111111111111111",
                        "rentEpoch": 0
                    })
                });
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "result": {
                        "context": { "slot": 1 },
                        "value": value
                    }
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(resp.as_bytes()).await.unwrap();
            }
        });
        format!("http://127.0.0.1:{port}/")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn decodes_idl_at_primary_pda() {
        let program_id = Pubkey::new_unique();
        let primary = anchor_idl_pda_primary(&program_id).to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let url = serve_accounts(
            HashMap::from([(primary.clone(), build_fixture_bytes())]),
            1,
            Arc::clone(&requests),
        )
        .await;

        let idl = fetch_anchor_idl(&url, &program_id.to_string())
            .await
            .unwrap()
            .expect("primary idl account");

        assert_eq!(idl.name, "hello_world");
        assert_eq!(requests.lock().unwrap().as_slice(), &[primary]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn falls_back_to_legacy_pda_when_primary_missing() {
        let program_id = Pubkey::new_unique();
        let primary = anchor_idl_pda_primary(&program_id).to_string();
        let legacy = anchor_idl_pda_legacy(&program_id).unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let url = serve_accounts(
            HashMap::from([(legacy.clone(), build_fixture_bytes())]),
            2,
            Arc::clone(&requests),
        )
        .await;

        let idl = fetch_anchor_idl(&url, &program_id.to_string())
            .await
            .unwrap()
            .expect("legacy idl account");

        assert_eq!(idl.name, "hello_world");
        assert_eq!(requests.lock().unwrap().as_slice(), &[primary, legacy]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_idl_accounts_return_none() {
        let program_id = Pubkey::new_unique();
        let primary = anchor_idl_pda_primary(&program_id).to_string();
        let legacy = anchor_idl_pda_legacy(&program_id).unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let url = serve_accounts(
            HashMap::from([(primary.clone(), vec![9u8; 10])]),
            2,
            Arc::clone(&requests),
        )
        .await;

        let idl = fetch_anchor_idl(&url, &program_id.to_string())
            .await
            .unwrap();

        assert!(idl.is_none());
        assert_eq!(requests.lock().unwrap().as_slice(), &[primary, legacy]);
    }

    #[test]
    fn writes_f2_pda_derivation_evidence() {
        let program_id = Pubkey::from_str("4eoZHrR7VEJ1YonxjERYp7Cw95eSYuxpfDL1NtShe42d").unwrap();
        let evidence = serde_json::json!({
            "program_id": program_id.to_string(),
            "primary_pda": anchor_idl_pda_primary(&program_id).to_string(),
            "legacy_pda": anchor_idl_pda_legacy(&program_id).unwrap().to_string(),
            "match_form": "primary_v0_30_with_legacy_fallback"
        });
        fs::create_dir_all("/tmp/m6-evidence").unwrap();
        fs::write(
            "/tmp/m6-evidence/f2-pda-derivation.json",
            serde_json::to_string_pretty(&evidence).unwrap() + "\n",
        )
        .unwrap();
    }

    fn build_fixture_for(idl_json: &serde_json::Value) -> Vec<u8> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&serde_json::to_vec(idl_json).unwrap())
            .unwrap();
        let compressed = enc.finish().unwrap();
        let mut buf = vec![0u8; HEADER_LEN];
        buf[HEADER_LEN - 4..].copy_from_slice(&(compressed.len() as u32).to_le_bytes());
        buf.extend_from_slice(&compressed);
        buf
    }

    #[test]
    fn discriminator_from_idl_v030() {
        // F3.3: when the IDL JSON carries an explicit v0.30+ `discriminator`,
        // decode preserves the byte array verbatim instead of recomputing it.
        let idl_json = serde_json::json!({
            "version": "0.1.0", "name": "fake",
            "instructions": [{
                "name": "fake_ix",
                "discriminator": [1, 2, 3, 4, 5, 6, 7, 8],
                "args": []
            }]
        });
        let idl = decode_anchor_idl_payload(&build_fixture_for(&idl_json)).expect("decode");
        assert_eq!(
            idl.instructions[0].discriminator,
            Some([1, 2, 3, 4, 5, 6, 7, 8])
        );
    }

    #[test]
    fn discriminator_fallback_sha256() {
        // F3.4: canonical Anchor sha256("global:initialize")[..8] value.
        assert_eq!(
            compute_anchor_discriminator("initialize"),
            [175, 175, 109, 31, 13, 152, 155, 237]
        );
    }

    #[test]
    fn decode_anchor_idl_v030_with_discriminator_and_accounts() {
        let idl_json = serde_json::json!({
            "version": "0.1.0", "name": "hello_world",
            "address": "HELLO111111111111111111111111111111111111111",
            "instructions": [{
                "name": "greet", "discriminator": [10,20,30,40,50,60,70,80],
                "accounts": [{ "name": "user", "isMut": true, "isSigner": true }],
                "args": [{ "name": "name", "type": "string" }]
            }]
        });
        let idl = decode_anchor_idl_payload(&build_fixture_for(&idl_json)).expect("decode");
        assert_eq!(
            idl.address.as_deref(),
            Some("HELLO111111111111111111111111111111111111111")
        );
        let ix = &idl.instructions[0];
        assert_eq!(ix.discriminator, Some([10, 20, 30, 40, 50, 60, 70, 80]));
        assert_eq!(ix.accounts.len(), 1);
        assert!(ix.accounts[0].is_mut && ix.accounts[0].is_signer);
    }
}
