//! IdlRegistry — in-memory DashMap of Programs, IDLs, and synthesized Skills.
//!
//! Consumes a `Stream<YellowstoneEvent>` (real Yellowstone gRPC in production,
//! `MockYellowstoneStream` in tests) and for every `ProgramDeployed` event it
//! looks up an Anchor IDL (mock table only in the Spike), synthesizes Skills
//! via `skill_synth`, and stores all three in DashMap tables.

use std::sync::Arc;

use dashmap::DashMap;
use futures::stream;
use tokio_stream::{Stream, StreamExt};

pub use skill_synth::{Idl, IdlInstruction, IdlInstructionArg, Program, Skill};

#[cfg(feature = "live-yellowstone")]
pub mod yellowstone;

pub mod anchor_idl;
pub mod native_skills;

pub use native_skills::register_spl_token_transfer_skill;

/// Events that can be consumed by `IdlRegistry::attach_stream`.
#[derive(Clone, Debug)]
pub enum YellowstoneEvent {
    ProgramDeployed { program_id: String },
}

/// Thread-safe, cloneable handle to the shared in-memory registry tables.
#[derive(Clone, Default)]
pub struct IdlRegistry {
    pub programs: Arc<DashMap<String, Program>>,
    pub idls: Arc<DashMap<String, Idl>>,
    pub skills: Arc<DashMap<String, Skill>>,
    /// Mock Anchor IDL table used by the Spike integration tests and demo.
    /// Keyed by program_id → IDL. Production will be replaced by RPC lookup.
    pub mock_idls: Arc<DashMap<String, Idl>>,
    /// Optional Solana JSON-RPC endpoint used when no mock IDL is found.
    pub rpc_url: Option<String>,
}

impl IdlRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a registry that falls back to an on-chain Anchor IDL fetch
    /// when no mock is registered. The mock path still wins when both exist.
    pub fn with_rpc_url(url: impl Into<String>) -> Self {
        Self {
            rpc_url: Some(url.into()),
            ..Self::default()
        }
    }

    /// Insert a mock IDL that will be served by `try_fetch_anchor_idl`.
    /// Tests and the demo use this to simulate Anchor IDL PDA fetches.
    pub fn insert_mock_idl(&self, program_id: &str, idl: Idl) {
        self.mock_idls.insert(program_id.to_string(), idl);
    }

    /// Snapshot of currently known skills.
    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.iter().map(|e| e.value().clone()).collect()
    }

    /// Lookup a stored IDL by program_id.
    pub fn get_idl(&self, program_id: &str) -> Option<Idl> {
        self.idls.get(program_id).map(|e| e.value().clone())
    }

    /// True if a skill by this id exists.
    pub fn has_skill(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    /// Consume a Yellowstone event stream in a background task. The spawned
    /// task exits when the stream ends. Returns immediately.
    pub fn attach_stream<S>(self: &Arc<Self>, mut stream: S) -> tokio::task::JoinHandle<()>
    where
        S: Stream<Item = YellowstoneEvent> + Send + Unpin + 'static,
    {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(ev) = stream.next().await {
                this.handle_event(ev).await;
            }
        })
    }

    /// Process a single event; exposed for synchronous integration tests.
    pub async fn handle_event(&self, ev: YellowstoneEvent) {
        match ev {
            YellowstoneEvent::ProgramDeployed { program_id } => {
                tracing::info!(event = "program_discovered", %program_id, "new program observed");
                match self.try_fetch_anchor_idl(&program_id).await {
                    Some(idl) => {
                        tracing::info!(event = "idl_fetched", %program_id, "anchor idl fetched");
                        self.idls.insert(program_id.clone(), idl.clone());
                        let program = Program {
                            id: program_id.clone(),
                            name: Some(idl.name.clone()),
                        };
                        self.programs.insert(program_id.clone(), program.clone());
                        tracing::info!(
                            event = "idl_decoded",
                            %program_id,
                            instructions = idl.instructions.len(),
                            "idl decoded"
                        );
                        for skill in skill_synth::synthesize(&program, &idl) {
                            tracing::info!(
                                event = "skill_synthesized",
                                skill_id = %skill.skill_id,
                                "skill added"
                            );
                            self.skills.insert(skill.skill_id.clone(), skill);
                        }
                    }
                    None => {
                        tracing::debug!(%program_id, "no anchor idl; skipping");
                    }
                }
            }
        }
    }

    /// Lazy cache-miss path used by the proxy when a skill is absent from the
    /// in-memory registry. When `rpc_url` is Some and no `<program_id>::*`
    /// skill currently exists, fetches the on-chain Anchor IDL, synthesizes
    /// skills, and inserts them. Returns `Ok(true)` if any were added.
    pub async fn try_fetch_and_register(&self, program_id: &str) -> anyhow::Result<bool> {
        if self.rpc_url.is_none() {
            return Ok(false);
        }
        let prefix = format!("{}::", program_id);
        if self.skills.iter().any(|e| e.key().starts_with(&prefix)) {
            return Ok(false);
        }
        let url = self.rpc_url.as_deref().unwrap();
        let Some(idl) = anchor_idl::fetch_anchor_idl(url, program_id).await? else {
            return Ok(false);
        };
        self.idls.insert(program_id.to_string(), idl.clone());
        let program = Program { id: program_id.to_string(), name: Some(idl.name.clone()) };
        self.programs.insert(program_id.to_string(), program.clone());
        let mut added = 0usize;
        for skill in skill_synth::synthesize(&program, &idl) {
            self.skills.insert(skill.skill_id.clone(), skill);
            added += 1;
        }
        Ok(added > 0)
    }

    /// Anchor IDL lookup. Mock table always wins; if none is registered and
    /// `rpc_url` is configured, fall back to an on-chain fetch.
    pub async fn try_fetch_anchor_idl(&self, program_id: &str) -> Option<Idl> {
        if let Some(idl) = self.mock_idls.get(program_id).map(|e| e.value().clone()) {
            return Some(idl);
        }
        if let Some(url) = self.rpc_url.as_deref() {
            match anchor_idl::fetch_anchor_idl(url, program_id).await {
                Ok(opt) => return opt,
                Err(err) => {
                    tracing::warn!(%program_id, error = %err, "anchor idl rpc fetch failed");
                    return None;
                }
            }
        }
        None
    }
}

/// Test helper. Produces a `Stream<YellowstoneEvent>` from a vec.
pub struct MockYellowstoneStream;
impl MockYellowstoneStream {
    pub fn new(events: Vec<YellowstoneEvent>) -> impl Stream<Item = YellowstoneEvent> + Unpin {
        Box::pin(stream::iter(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_idl() -> Idl {
        Idl {
            version: "0.1.0".into(),
            name: "hello_world".into(),
            instructions: vec![
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

    #[tokio::test]
    async fn attach_stream_populates_skills() {
        let registry = Arc::new(IdlRegistry::new());
        let pid = "HELLO111111111111111111111111111111111111111";
        registry.insert_mock_idl(pid, sample_idl());

        let stream = MockYellowstoneStream::new(vec![
            YellowstoneEvent::ProgramDeployed { program_id: pid.into() },
        ]);
        let handle = registry.attach_stream(stream);
        handle.await.unwrap();

        let skills = registry.list_skills();
        assert_eq!(skills.len(), 2);
        assert!(skills.iter().any(|s| s.instruction_name == "greet"));
    }

    #[tokio::test]
    async fn missing_idl_is_skipped_without_panic() {
        let registry = Arc::new(IdlRegistry::new());
        let stream = MockYellowstoneStream::new(vec![
            YellowstoneEvent::ProgramDeployed { program_id: "UNKNOWN".into() },
        ]);
        registry.attach_stream(stream).await.unwrap();
        assert!(registry.list_skills().is_empty());
    }

    #[tokio::test]
    async fn mock_wins_over_rpc() {
        // rpc_url points at an unroutable TCP port; if the mock path is ever
        // bypassed the fetch would error, but we assert skills are populated.
        let registry = Arc::new(IdlRegistry::with_rpc_url("http://127.0.0.1:1/"));
        let pid = "HELLO111111111111111111111111111111111111111";
        registry.insert_mock_idl(pid, sample_idl());
        let stream = MockYellowstoneStream::new(vec![
            YellowstoneEvent::ProgramDeployed { program_id: pid.into() },
        ]);
        registry.attach_stream(stream).await.unwrap();
        let skills = registry.list_skills();
        assert_eq!(skills.len(), 2, "mock path must populate skills");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_path_used_when_no_mock() {
        use base64::Engine as _;
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let idl = serde_json::json!({"version":"0.1.0","name":"rpc_hello",
            "instructions":[{"name":"ping","args":[]}]});
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&serde_json::to_vec(&idl).unwrap()).unwrap();
        let zbody = enc.finish().unwrap();
        let mut acct = vec![0u8; 8];
        acct.extend_from_slice(&[1u8; 32]);
        acct.extend_from_slice(&(zbody.len() as u32).to_le_bytes());
        acct.extend_from_slice(&zbody);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&acct);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let _ = s.read(&mut [0u8; 4096]).await.unwrap();
            let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"context\":{{\"slot\":1}},\"value\":{{\"data\":[\"{}\",\"base64\"],\"executable\":false,\"lamports\":0,\"owner\":\"11111111111111111111111111111111\",\"rentEpoch\":0}}}}}}", b64);
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            s.write_all(resp.as_bytes()).await.unwrap();
        });
        let registry = Arc::new(IdlRegistry::with_rpc_url(format!("http://127.0.0.1:{}/", port)));
        let pid = "11111111111111111111111111111111";
        let stream = MockYellowstoneStream::new(vec![YellowstoneEvent::ProgramDeployed { program_id: pid.into() }]);
        registry.attach_stream(stream).await.unwrap();
        assert_eq!(registry.list_skills()[0].instruction_name, "ping");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lazy_fetch_populates_skills_from_rpc() {
        use base64::Engine as _;
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let idl = serde_json::json!({"version":"0.1.0","name":"lazy_hello",
            "instructions":[{"name":"ping","args":[]}]});
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&serde_json::to_vec(&idl).unwrap()).unwrap();
        let zbody = enc.finish().unwrap();
        let mut acct = vec![0u8; 8];
        acct.extend_from_slice(&[1u8; 32]);
        acct.extend_from_slice(&(zbody.len() as u32).to_le_bytes());
        acct.extend_from_slice(&zbody);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&acct);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let _ = s.read(&mut [0u8; 4096]).await.unwrap();
            let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"context\":{{\"slot\":1}},\"value\":{{\"data\":[\"{}\",\"base64\"],\"executable\":false,\"lamports\":0,\"owner\":\"11111111111111111111111111111111\",\"rentEpoch\":0}}}}}}", b64);
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            s.write_all(resp.as_bytes()).await.unwrap();
        });
        let registry = IdlRegistry::with_rpc_url(format!("http://127.0.0.1:{}/", port));
        let pid = "11111111111111111111111111111111";
        assert!(!registry.has_skill(&format!("{}::ping", pid)));
        let added = registry.try_fetch_and_register(pid).await.unwrap();
        assert!(added, "expected lazy fetch to add skills");
        assert!(registry.has_skill(&format!("{}::ping", pid)));
    }
}
