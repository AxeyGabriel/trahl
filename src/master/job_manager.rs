use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::master::peers::{PeerId, RxManagerMsg};
use crate::rpc::{TranscodeProgress, WorkerInfo};
use super::MasterCtx;
use super::socket_server::SocketEvent;
use super::peers::TxManagerMsg;
use crate::rpc::Message;
use crate::rpc::JobStatus as RpcJobStatus;

fn uuid_to_u128(value: Uuid) -> u128 {
    u128::from_be_bytes(*value.as_bytes())
}

fn u128_to_uuid(value: u128) -> Uuid {
    Uuid::from_bytes(value.to_be_bytes())
}

struct PeerInfo {
    tx: mpsc::Sender<RxManagerMsg>, // To send message to peer
    info: WorkerInfo,
    jobs: HashMap<Uuid, JobTracking>
}

enum JobStatus {
    Sent,
    Acknowledged,
    InProgress(TranscodeProgress),
    Finished,
    Failed(String),
    Success,
}

struct JobTracking { 
    id: Uuid,
    log: Vec<String>,
    status: JobStatus
}

pub struct JobManager {
    rx_from_peer: mpsc::Receiver<TxManagerMsg>,
    rx_socket_events: mpsc::Receiver<SocketEvent>,
    peer_registry: HashMap<PeerId, PeerInfo>,
// pending_jobs: VecDeque<JobSpec>
// running_jobs: HashMap<Uuid, PeerId>
}

impl JobManager {
    pub fn new(
        rx_from_peer: mpsc::Receiver<TxManagerMsg>,
        rx_socket_events: mpsc::Receiver<SocketEvent>,
    ) -> Self {
        Self {
            rx_from_peer,
            rx_socket_events,
            peer_registry: HashMap::new(),
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let mut ch_reload = ctx.ch_reload.1.clone();

        loop {
            tokio::select!(
                Some((peer_id, msg)) = self.rx_from_peer.recv() => {
                    if let Some(peer) = self.peer_registry.get_mut(&peer_id) {
                        self.msg_from_peer(peer, msg);
                    } else {
                        warn!("Message received from unknown peer");
                    }
                },
                Some(event) = self.rx_socket_events.recv() => {
                    match event {
                        SocketEvent::PeerConnected(peer_id, tx) => {
                            let peer_info = PeerInfo {
                                tx
                            };
                            self.peer_registry
                                .insert(peer_id, peer_info);
                        },
                        SocketEvent::PeerDisconnected(peer_id) => {
                            self.peer_registry
                                .remove(&peer_id);
                        }
                    }
                },
                _ = ch_reload.changed() => {
                    if *ch_reload.borrow() {
                        //todo! send configupdate
                    }
                },
                _ = ch_term.changed() => {
                    if *ch_term.borrow() {
                        break;
                    }
                }
            );
        }
    }

    async fn msg_from_peer(&mut self, peer: &mut PeerInfo, msg: Message) {
        match msg {
            Message::JobStatus(msg) => {
                let job_id = &u128_to_uuid(msg.job_id);
                if let Some(job_tracking) = peer.jobs.get_mut(job_id) {
                    match msg.status {
                        RpcJobStatus::Ack => {
                            job_tracking.status = JobStatus::Acknowledged;
                        },
                        RpcJobStatus::Progress(p) => {
                            job_tracking.status = JobStatus::InProgress(p);
                        },
                        RpcJobStatus::Log {line} => {
                            job_tracking.log
                                .push(line);
                        },
                        RpcJobStatus::Error {descr} => {
                            job_tracking.status = JobStatus::Failed(descr);

                        },
                        RpcJobStatus::Done => {
                            job_tracking.status = JobStatus::Success;
                        }
                    }
                } else {
                    warn!("Received updates for a unknown job");
                }

            },
            _ => {}
        }
    }
}

