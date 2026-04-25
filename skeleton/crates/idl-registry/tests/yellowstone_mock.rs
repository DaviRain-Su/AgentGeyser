#![cfg(feature = "live-yellowstone")]

use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::{Stream, StreamExt};
use idl_registry::{
    yellowstone::{connect_stream, YellowstoneConfig},
    Idl, IdlInstruction, IdlRegistry, YellowstoneEvent,
};
use solana_sdk::pubkey::Pubkey;
use yellowstone_grpc_proto::{
    prelude::*,
    tonic::{self, Request, Response, Status},
};

const BPF_LOADER_UPGRADEABLE: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

type MockSubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeUpdate, Status>> + Send>>;

struct MockGeyser {
    updates: Arc<Vec<SubscribeUpdate>>,
    emitted_count: Arc<AtomicUsize>,
}

#[rustfmt::skip]
#[tonic::async_trait]
impl geyser_server::Geyser for MockGeyser {
    type SubscribeStream = MockSubscribeStream;

    async fn subscribe(
        &self,
        _request: Request<tonic::Streaming<SubscribeRequest>>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        self.emitted_count
            .fetch_add(self.updates.len(), Ordering::SeqCst);
        let updates = (*self.updates).clone().into_iter().map(Ok);
        Ok(Response::new(Box::pin(futures::stream::iter(updates))))
    }

    async fn subscribe_replay_info(&self, _request: Request<SubscribeReplayInfoRequest>) -> Result<Response<SubscribeReplayInfoResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PongResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn get_latest_blockhash(&self, _request: Request<GetLatestBlockhashRequest>) -> Result<Response<GetLatestBlockhashResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn get_block_height(&self, _request: Request<GetBlockHeightRequest>) -> Result<Response<GetBlockHeightResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn get_slot(&self, _request: Request<GetSlotRequest>) -> Result<Response<GetSlotResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn is_blockhash_valid(&self, _request: Request<IsBlockhashValidRequest>) -> Result<Response<IsBlockhashValidResponse>, Status> { Err(Status::unimplemented("mock")) }
    async fn get_version(&self, _request: Request<GetVersionRequest>) -> Result<Response<GetVersionResponse>, Status> { Err(Status::unimplemented("mock")) }
}

struct MockServer {
    addr: std::net::SocketAddr,
    emitted_count: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_mock_geyser(updates: Vec<SubscribeUpdate>) -> MockServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let emitted_count = Arc::new(AtomicUsize::new(0));
    let service = MockGeyser {
        updates: Arc::new(updates),
        emitted_count: Arc::clone(&emitted_count),
    };
    let incoming = futures::stream::unfold(listener, |listener| async {
        match listener.accept().await {
            Ok((socket, _addr)) => Some((Ok::<_, std::io::Error>(socket), listener)),
            Err(err) => Some((Err(err), listener)),
        }
    });
    let task = tokio::spawn(async move {
        let _ = tonic::transport::Server::builder()
            .add_service(geyser_server::GeyserServer::new(service))
            .serve_with_incoming(incoming)
            .await;
    });
    MockServer {
        addr,
        emitted_count,
        task,
    }
}

fn program_update(program_id: Pubkey) -> SubscribeUpdate {
    SubscribeUpdate {
        filters: vec!["ag-program-deploys".into()],
        update_oneof: Some(subscribe_update::UpdateOneof::Account(
            SubscribeUpdateAccount {
                account: Some(SubscribeUpdateAccountInfo {
                    pubkey: program_id.to_bytes().to_vec(),
                    lamports: 1,
                    owner: bs58::decode(BPF_LOADER_UPGRADEABLE).into_vec().unwrap(),
                    executable: true,
                    rent_epoch: 0,
                    data: program_account_data([2, 0, 0, 0]),
                    write_version: 1,
                    txn_signature: Some(vec![7; 64]),
                }),
                slot: 1,
                is_startup: false,
            },
        )),
        ..Default::default()
    }
}

fn non_program_update(program_id: Pubkey) -> SubscribeUpdate {
    let mut update = program_update(program_id);
    if let Some(subscribe_update::UpdateOneof::Account(account)) = &mut update.update_oneof {
        account.account.as_mut().unwrap().data = program_account_data([3, 0, 0, 0]);
    }
    update
}

fn program_account_data(prefix: [u8; 4]) -> Vec<u8> {
    let mut data = vec![0u8; 36];
    data[..4].copy_from_slice(&prefix);
    data[4..].copy_from_slice(&[9u8; 32]);
    data
}

fn sample_idl() -> Idl {
    Idl {
        version: "0.1.0".into(),
        name: "mock_anchor".into(),
        instructions: vec![IdlInstruction {
            name: "ping".into(),
            args: vec![],
            ..Default::default()
        }],
        ..Default::default()
    }
}

async fn connect_to_mock(server: &MockServer) -> impl Stream<Item = YellowstoneEvent> + Unpin {
    connect_stream(YellowstoneConfig {
        endpoint: format!("http://{}", server.addr),
        token: None,
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_stream_emits_program_deployed_on_account_write() {
    let program_id = Pubkey::new_from_array([42; 32]);
    let server = start_mock_geyser(vec![program_update(program_id)]).await;
    let mut stream = connect_to_mock(&server).await;

    let event = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap();

    assert_program_deployed(event, &program_id.to_string());

    std::fs::create_dir_all("/tmp/m6-evidence").unwrap();
    std::fs::write(
        "/tmp/m6-evidence/f5-mock-tonic.json",
        format!(
            "{}\n",
            serde_json::json!({
                "mock_addr": server.addr.to_string(),
                "update_count": server.emitted_count.load(Ordering::SeqCst),
                "parsed_event_count": 1
            })
        ),
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn filter_drops_non_program_writes() {
    let bad_program_id = Pubkey::new_from_array([7; 32]);
    let good_program_id = Pubkey::new_from_array([8; 32]);
    let server = start_mock_geyser(vec![
        non_program_update(bad_program_id),
        program_update(good_program_id),
    ])
    .await;
    let mut stream = connect_to_mock(&server).await;

    let event = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap();

    assert_program_deployed(event, &good_program_id.to_string());
}

fn assert_program_deployed(event: YellowstoneEvent, expected_program_id: &str) {
    match event {
        YellowstoneEvent::ProgramDeployed { program_id } => {
            assert_eq!(program_id, expected_program_id);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_register_skill_on_program_deployed_from_mock_geyser() {
    let program_id = Pubkey::new_from_array([99; 32]);
    let server = start_mock_geyser(vec![program_update(program_id)]).await;
    let stream = connect_to_mock(&server).await;
    let registry = Arc::new(IdlRegistry::new());
    registry.insert_mock_idl(&program_id.to_string(), sample_idl());
    let handle = registry.attach_stream(stream);

    let skills = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let skills = registry.list_skills();
            if !skills.is_empty() {
                return skills;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    assert!(skills
        .iter()
        .any(|skill| skill.program_id == program_id.to_string()));
    handle.abort();
}
