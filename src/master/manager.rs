pub mod events;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use rand::Rng;
use tokio::time::{interval, Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use super::MasterCtx;
use crate::master::manager::events::JobQueueEntry;
use crate::master::peers::{PeerId, RxManagerMsg};
use crate::rpc::WorkerInfo;
use super::socket_server::SocketEvent;
use super::peers::TxManagerMsg;
use crate::rpc::JobStatus as RpcJobStatus;
use crate::rpc::{JobMsg, Message};
use crate::utils;
use events::ManagerEvent;

struct PeerInfo {
    tx: mpsc::Sender<RxManagerMsg>, // To send message to peer
    info: WorkerInfo,
    jobs: HashMap<Uuid, JobTracking>
}

impl PeerInfo {
    fn active_jobs(&self) -> Vec<&JobTracking> {
        self.jobs
            .values()
            .filter(|j| matches!(
                j.status,
                JobStatus::Sent | JobStatus::Running
            ))
            .collect()
    }
}

enum JobStatus {
    Sent,
    Running,
    Ended,
}

struct JobTracking {
    contract: JobContract,
    events: Vec<RpcJobStatus>,
    status: JobStatus
}

#[derive(Clone)]
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
    tx_events: broadcast::Sender<ManagerEvent>,
    rx_from_peer: mpsc::Receiver<TxManagerMsg>,
    rx_socket_events: mpsc::Receiver<SocketEvent>,
    peer_registry: HashMap<PeerId, PeerInfo>,
    rx_job: mpsc::Receiver<JobContract>,
    pending_jobs: VecDeque<JobContract>,
    dedup_jobs: HashSet<PathBuf>,
}

impl JobManager {
    pub fn new(
        rx_from_peer: mpsc::Receiver<TxManagerMsg>,
        rx_socket_events: mpsc::Receiver<SocketEvent>,
        rx_job: mpsc::Receiver<JobContract>,
        tx_events: broadcast::Sender<ManagerEvent>,
    ) -> Self {
        Self {
            rx_from_peer,
            rx_socket_events,
            peer_registry: HashMap::new(),
            pending_jobs: VecDeque::new(),
            dedup_jobs: HashSet::new(),
            rx_job,
            tx_events,
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let mut ch_reload = ctx.ch_reload.1.clone();
        let mut dispatch_timer = interval(Duration::from_secs(1));
        let mut sse_timer = interval(Duration::from_millis(200));

        loop {
            tokio::select!(
                _ = sse_timer.tick() => {
                    let mut rng = rand::rng();
                    let value: f64 = rng.random_range(0.0..=100.0);
                    let me = ManagerEvent::JobQueue(vec![
                        JobQueueEntry {
                            file: "abc.mp4".to_string(),
                            library: "Movies".to_string(),
                            worker: "-".to_string(),
                            status: "QUEUED".to_string(),
                            milestone: "-".to_string(),
                            progress: "-".to_string(),
                            eta: "-".to_string(),
                        },
                        JobQueueEntry {
                            file: "HIMYM.S01E01.h264.1080p.mkv".to_string(),
                            library: "Movies".to_string(),
                            worker: "worker-01".to_string(),
                            status: "PROCESSING".to_string(),
                            milestone: "TRANSCODING".to_string(),
                            progress: format!("{:.1}%", value),
                            eta: "00:14:35".to_string(),
                        },
                    ]);
                    _ = self.tx_events.send(me);
                },
                _ = dispatch_timer.tick() => {
                    let selected_peer = self.peer_registry
                        .iter_mut()
                        .filter(|(_, p)| p.active_jobs().len() < p.info.simultaneous_jobs.into())
                        .min_by_key(|(_, p)| p.active_jobs().len());

                    if let Some((_id, peer)) = selected_peer {
                        if let Some(job) = self.pending_jobs.pop_front() {
                            let script = tokio::fs::read_to_string(job.script_path.clone()).await.unwrap();
                            let src_file_clone = job.src_file.clone();
                            let jobmsg = JobMsg {
                                job_id: utils::uuid_to_u128(job.id),
                                script: script,
                                vars: job.vars.clone(),
                                file: job.src_file.clone().into_os_string().to_string_lossy().into_owned(),
                                dst_dir: job.dst_dir.clone().to_string_lossy().to_string(),
                                library_root: job.library_root.clone().to_string_lossy().into_owned(),
                            };

                            info!("Sent job id {} to worker {}", jobmsg.job_id, peer.info.identifier);
                            
                            let msg = Message::job(jobmsg); 
                            _ = peer.tx.send(msg).await;

                            peer.jobs.insert(job.id, JobTracking {
                                events: Vec::new(),
                                status: JobStatus::Sent,
                                contract: job,
                            });
                            self.dedup_jobs.remove(&src_file_clone);
                        }
                    }
                },
                Some(job) = self.rx_job.recv() => {
                    self.dedup_schedule_job(job).await;
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
                            self.peer_registry.insert(peer_id, peer_info);
                        },
                        SocketEvent::PeerDisconnected(peer_id) => {
                            if let Some(peer) = self.peer_registry.get(&peer_id) {
                                let active_jobs: Vec<JobContract> = peer.active_jobs()
                                    .iter()
                                    .map(|job| job.contract.clone())
                                    .collect();
                                for job in active_jobs {
                                    self.dedup_schedule_job(job).await;
                                }
                            }
                            self.peer_registry.remove(&peer_id);
                        }
                    }
                },
                _ = ch_reload.changed() => {
                    if *ch_reload.borrow() {
                        //todo! send configupdate
                        //maybe cancel all jobs and resend
                    }
                },
                _ = ch_term.changed() => {
                    if *ch_term.borrow() {
                        for (_, peer) in self.peer_registry {
                            let _ = peer.tx.send(Message::cancel_jobs()).await;
                            //todo improve this
                        }
                        break;
                    }
                }
            );
        }
    }

    async fn dedup_schedule_job(&mut self, job: JobContract) {
        if self.dedup_jobs.insert(job.src_file.clone()) {
            info!("Received job for file {}", job.src_file.display());
            self.pending_jobs.push_back(job);
        } else {
            debug!("Skipping duplicated job for {}", job.src_file.display());
        }
    }
}

async fn msg_from_peer(peer: &mut PeerInfo, msg: Message) {
    match msg {
        Message::JobStatus(msg) => {
            let job_id = &utils::u128_to_uuid(msg.job_id);
            if let Some(job_tracking) = peer.jobs.get_mut(job_id) {
                job_tracking.events
                    .push(msg.status.clone());

                match msg.status {
                    RpcJobStatus::Ack => {
                        debug!("Job {} ack on worker {}", msg.job_id, peer.info.identifier);
                        job_tracking.status = JobStatus::Running;
                    },
                    RpcJobStatus::Declined(reason) => {
                        debug!("Job {} declined on worker {}: {}", msg.job_id, peer.info.identifier, reason);
                        job_tracking.status = JobStatus::Ended;
                    },
                    RpcJobStatus::Progress(p) => {
                        debug!("Job {} progress: {:?} eta: {:?}", msg.job_id, p.percentage, p.eta);
                    },
                    RpcJobStatus::Log(line) => {
                        debug!("Job {} log: {}", msg.job_id, line);
                    },
                    RpcJobStatus::Milestone(descr) => {
                        info!("Job {} milestone: {}", msg.job_id, descr);
                    },
                    RpcJobStatus::Error(descr) => {
                        error!("Job {} failed on worker {}: {}", msg.job_id, peer.info.identifier, descr);
                        job_tracking.status = JobStatus::Ended;
                        //todo! signal job discovery system
                    },
                    RpcJobStatus::Done { file } => {
                        info!("Job {} completed successfuly on worker {}, output={:?}", msg.job_id, peer.info.identifier, file);
                        job_tracking.status = JobStatus::Ended;
                        //todo! signal job discovery system
                    },
                    RpcJobStatus::Copying => {
                        info!("Job {} is copying files on worker {}", msg.job_id, peer.info.identifier);
                    },
                }
            } else {
                warn!("Received updates for a unknown job: {}", msg.job_id);
            }

        },
        _ => {}
    }
}
