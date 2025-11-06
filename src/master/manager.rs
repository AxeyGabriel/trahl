pub mod events;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio::sync::{broadcast, mpsc};
use tracing::{
    info,
    warn,
    error,
    debug,
    trace,
};
use anyhow::{Result, anyhow};
use sqlx::{
    Pool,
    Sqlite,
};

use super::MasterCtx;
use crate::master::peers::{PeerId, RxManagerMsg};
use crate::rpc::WorkerInfo;
use super::socket_server::SocketEvent;
use super::peers::TxManagerMsg;
use crate::rpc::JobStatus as RpcJobStatus;
use crate::rpc::{JobMsg, Message};
use events::ManagerEvent;
use super::db::{
    self,
    model::{
        Job,
        FileEntry,
        Library,
        Script,
        Variable,
    },
};
use std::sync::{
    OnceLock,
    Mutex,
};

static JOB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct PeerInfo {
    tx: mpsc::Sender<RxManagerMsg>, // To send message to peer
    info: WorkerInfo,
    jobs: HashMap<i64, JobTracking>
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
    id: i64,
    src_file: PathBuf,
    dst_dir: PathBuf,
    vars: HashMap<String, String>,
    script: String,
    library_root: PathBuf,
}

impl JobContract {
    pub fn new(id: i64, library_root: PathBuf, src_file: PathBuf, dst_dir: PathBuf, vars: HashMap<String, String>, script: String) -> Self {
        Self {
            id,
            src_file,
            dst_dir,
            vars,
            script,
            library_root,
        }
    }
}

pub struct JobManager {
    tx_events: broadcast::Sender<ManagerEvent>,
    rx_from_peer: mpsc::Receiver<TxManagerMsg>,
    rx_socket_events: mpsc::Receiver<SocketEvent>,
    peer_registry: HashMap<PeerId, PeerInfo>,
}

impl JobManager {
    pub fn new(
        rx_from_peer: mpsc::Receiver<TxManagerMsg>,
        rx_socket_events: mpsc::Receiver<SocketEvent>,
        tx_events: broadcast::Sender<ManagerEvent>,
    ) -> Self {
        Self {
            rx_from_peer,
            rx_socket_events,
            peer_registry: HashMap::new(),
            tx_events,
        }
    }

    pub async fn run(mut self, ctx: Arc<MasterCtx>) {
        let mut ch_term = ctx.ch_terminate.1.clone();
        let mut ch_reload = ctx.ch_reload.1.clone();
        let mut dispatch_timer = interval(Duration::from_secs(2));

        let pool = db::DB.get().unwrap();

        loop {
            tokio::select!(
                _ = dispatch_timer.tick() => {
                    let selected_peer = self.peer_registry
                        .iter_mut()
                        .filter(|(_, p)| p.active_jobs().len() < p.info.simultaneous_jobs.into())
                        .min_by_key(|(_, p)| p.active_jobs().len());

                    if let Some((_id, peer)) = selected_peer {
                        if let Ok(Some(job)) = build_job_from_db().await {
                            let jobmsg = JobMsg {
                                job_id: job.id,
                                script: job.script.clone(),
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
                        }
                    }
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
                            let identifier = info.identifier.clone();
                            let peer_info = PeerInfo {
                                tx,
                                info,
                                jobs: HashMap::new(),
                            };
                            self.peer_registry.insert(peer_id, peer_info);
                            db::upsert_worker(identifier.as_str()).await;
                        },
                        SocketEvent::PeerDisconnected(peer_id) => {
                            if let Some(peer) = self.peer_registry.get(&peer_id) {
                                let active_jobs: Vec<JobContract> = peer.active_jobs()
                                    .iter()
                                    .map(|job| job.contract.clone())
                                    .collect();
                                for job in active_jobs {
                                    let _ = sqlx::query!(
                                        r#"
                                        UPDATE job
                                        SET status = 'queued',
                                            started_at = NULL
                                        WHERE id = ?
                                        "#,
                                        job.id
                                    )
                                    .execute(pool)
                                    .await;
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
}

async fn msg_from_peer(peer: &mut PeerInfo, msg: Message) {
    let pool = db::DB.get().unwrap();

    match msg {
        Message::JobStatus(msg) => {
            let job_id = msg.job_id;
            if let Some(job_tracking) = peer.jobs.get_mut(&job_id) {
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
                        let _ = sqlx::query!(
                            r#"
                            UPDATE job
                            SET status = 'queued',
                                started_at = NULL
                            WHERE id = ?
                            "#,
                            job_id
                        )
                        .execute(pool)
                        .await;
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
                        let _ = sqlx::query!(
                            r#"
                            UPDATE job
                            SET status = 'failure'
                            WHERE id = ?
                            "#,
                            job_id
                        )
                        .execute(pool)
                        .await;
                    },
                    RpcJobStatus::Done { file } => {
                        info!("Job {} completed successfuly on worker {}, output={:?}", msg.job_id, peer.info.identifier, file);
                        job_tracking.status = JobStatus::Ended;
                        let _ = sqlx::query!(
                            r#"
                            UPDATE job
                            SET status = 'success'
                            WHERE id = ?
                            "#,
                            job_id
                        )
                        .execute(pool)
                        .await;
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

fn job_lock() -> &'static Mutex<()> {
    JOB_LOCK.get_or_init(|| Mutex::new(()))
}

async fn build_job_from_db() -> Result<Option<JobContract>> {
    let Ok(_guard) = job_lock().try_lock() else {
        info!("Another instance of build_job_from_db is already running");
        return Err(anyhow!("resource locked"));
    };

    let pool = db::DB.get().unwrap();

    let res =  sqlx::query_as!(
        Job,
        r#"
        SELECT * FROM job
        WHERE status = 'queued'
        ORDER BY created_at ASC
        LIMIT 1
        "#
    )
    .fetch_optional(pool)
    .await?;

    let job = match res {
        Some(res) => {
            res
        },
        None => {
            trace!("No pending jobs found");
            return Ok(None);
        }
    };
   
    let file = sqlx::query_as!(
        FileEntry,
        r#"
        SELECT * FROM file_entry WHERE id = ?
        "#,
        job.file_id
    )
    .fetch_one(pool)
    .await?;
    
    let library = sqlx::query_as!(
        Library,
        r#"
        SELECT * FROM library WHERE id = ?
        "#,
        file.library_id
    )
    .fetch_one(pool)
    .await?;

    let script = sqlx::query_as!(
        Script,
        r#"
        SELECT * FROM script WHERE id = ?
        "#,
        library.script_id
    )
    .fetch_one(pool)
    .await?;

    let variables = sqlx::query_as!(
        Variable,
        r#"
        SELECT * FROM variables
        WHERE library_id IS NULL
        OR library_id = ?
        "#,
        library.id
    )
    .fetch_all(pool)
    .await?;

    let variables_map: HashMap<String, String> = variables
        .into_iter()
        .filter_map(|v| {
            v.value.map(|val| (v.key, val))
        }).collect();


    sqlx::query!(
        r#"
        UPDATE job
        SET status = 'processing',
            started_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
        job.id
    )
    .execute(pool)
    .await?;


    let abs_path = PathBuf::from(&library.path).join(&file.file_path);

    let jc = JobContract::new(
        job.id,
        library.path.into(),
        abs_path,
        library.destination.into(),
        variables_map,
        script.script,
    );
        
    Ok(Some(jc))
}
