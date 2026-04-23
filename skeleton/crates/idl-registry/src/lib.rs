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
}

impl IdlRegistry {
    pub fn new() -> Self {
        Self::default()
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

    /// Mock-only Anchor IDL lookup. Production would query the Anchor IDL PDA
    /// via RPC; the Spike only consults `mock_idls`.
    pub async fn try_fetch_anchor_idl(&self, program_id: &str) -> Option<Idl> {
        self.mock_idls.get(program_id).map(|e| e.value().clone())
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
                    args: vec![IdlInstructionArg { name: "name".into(), kind: "string".into() }],
                },
                IdlInstruction {
                    name: "set_counter".into(),
                    args: vec![IdlInstructionArg { name: "value".into(), kind: "u64".into() }],
                },
            ],
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
}
