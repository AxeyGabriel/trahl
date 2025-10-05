use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::master::peers::{PeerId, RxManagerMsg};
use crate::rpc::{TranscodeProgress, WorkerInfo};
use super::MasterCtx;
use super::socket_server::SocketEvent;
use super::peers::TxManagerMsg;
use crate::rpc::Message;
use crate::rpc::JobStatus as RpcJobStatus;
use crate::rpc::JobMsg;

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

impl PeerInfo {
    fn active_job_count(&self) -> usize {
        self.jobs
            .values()
            .filter(|j| matches!(
                j.status,
                JobStatus::Sent | JobStatus::Acknowledged | JobStatus::InProgress(_)
            ))
            .count()
    }
}

enum JobStatus {
    Sent,
    Acknowledged,
    InProgress(TranscodeProgress),
    Failed(String),
    Copying,
    Success,
}

struct JobTracking { 
    log: Vec<String>,
    status: JobStatus
}

pub struct JobContract {
    id: Uuid,
    src_file: PathBuf,
    dst_dir: PathBuf,
    vars: HashMap<String, String>,
    script_path: PathBuf,
    library_root: PathBuf,
}

impl JobContract {
    pub fn new(library_root: PathBuf, src_file: PathBuf, dst_dir: PathBuf, vars: HashMap<String, String>, script_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            src_file,
            dst_dir,
            vars,
            script_path,
            library_root,
        }
    }
}

pub struct JobManager {
    rx_from_peer: mpsc::Receiver<TxManagerMsg>,
    rx_socket_events: mpsc::Receiver<SocketEvent>,
    peer_registry: HashMap<PeerId, PeerInfo>,
    rx_job: mpsc::Receiver<JobContract>,
    pending_jobs: VecDeque<JobContract>
}

impl JobManager {
    pub fn new(
        rx_from_peer: mpsc::Receiver<TxManagerMsg>,
        rx_socket_events: mpsc::Receiver<SocketEvent>,
        rx_job: mpsc::Receiver<JobContract>
    ) -> Self {
        Self {
            rx_from_peer,
            rx_socket_events,
            peer_registry: HashMap::new(),
            pending_jobs: VecDeque::new(),
            rx_job,
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let mut ch_reload = ctx.ch_reload.1.clone();
        let mut dispatch_timer = interval(Duration::from_secs(1));

        loop {
            tokio::select!(
                _ = dispatch_timer.tick() => {
                    if let Some(job) = self.pending_jobs.pop_front() {
                        let selected_peer = self.peer_registry
                            .iter_mut()
                            .filter(|(_, p)| p.active_job_count() < p.info.simultaneous_jobs.into())
                            .min_by_key(|(_, p)| p.active_job_count());

                        if let Some((_id, peer)) = selected_peer {
                            let script = tokio::fs::read_to_string(job.script_path).await.unwrap();
                            let jobmsg = JobMsg {
                                job_id: uuid_to_u128(job.id),
                                script: script,
                                vars: job.vars,
                                file: job.src_file.into_os_string().to_string_lossy().into_owned(),
                                dst_dir: job.dst_dir.to_string_lossy().to_string(),
                                library_root: job.library_root.to_string_lossy().into_owned(),
                            };

                            info!("Sent job id {} to worker {}", jobmsg.job_id, peer.info.identifier);
                            
                            let msg = Message::Job(jobmsg); 
                            _ = peer.tx.send(msg).await;

                            peer.jobs.insert(job.id, JobTracking { log: Vec::new(), status: JobStatus::Sent });
                        } else {
                            self.pending_jobs.push_front(job);
                        }
                    }
                },
                Some(job) = self.rx_job.recv() => {
                    self.pending_jobs.push_back(job);
                },
                Some((peer_id, msg)) = self.rx_from_peer.recv() => {
                    if let Some(peer) = self.peer_registry.get_mut(&peer_id) {
                        msg_from_peer(peer, msg).await;
                    } else {
                        warn!("Message received from unknown peer");
                    }
                },
                Some(event) = self.rx_socket_events.recv() => {
                    match event {
                        SocketEvent::PeerConnected(peer_id, tx, info) => {
                            let peer_info = PeerInfo {
                                tx,
                                info,
                                jobs: HashMap::new(),
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
}

async fn msg_from_peer(peer: &mut PeerInfo, msg: Message) {
    match msg {
        Message::JobStatus(msg) => {
            let job_id = &u128_to_uuid(msg.job_id);
            if let Some(job_tracking) = peer.jobs.get_mut(job_id) {
                match msg.status {
                    RpcJobStatus::Ack => {
                        debug!("Job {} ack on worker {}", msg.job_id, peer.info.identifier);
                        job_tracking.status = JobStatus::Acknowledged;
                    },
                    RpcJobStatus::Progress(p) => {
                        debug!("Job {} progress: {:#?}", msg.job_id, p);
                        job_tracking.status = JobStatus::InProgress(p);
                    },
                    RpcJobStatus::Log {line} => {
                        debug!("Job {} log: {}", msg.job_id, line);
                        job_tracking.log
                            .push(line);
                    },
                    RpcJobStatus::Error {descr} => {
                        error!("Job {} failed on worker {}: {}", msg.job_id, peer.info.identifier, descr);
                        job_tracking.status = JobStatus::Failed(descr);

                    },
                    RpcJobStatus::Done { file } => {
                        info!("Job {} completed successfuly on worker {}, output={:?}", msg.job_id, peer.info.identifier, file);
                        job_tracking.status = JobStatus::Success;
                    },
                    RpcJobStatus::Copying => {
                        info!("Job {} is copying files on worker {}", msg.job_id, peer.info.identifier);
                        job_tracking.status = JobStatus::Copying;
                    },
                    _ => {}
                }
            } else {
                warn!("Received updates for a unknown job: {}", msg.job_id);
            }

        },
        _ => {}
    }
}
