use tokio::sync::mpsc;
use tokio::task::{self, JoinHandle};
use mlua::Lua;
use tracing::{info, error};

use crate::lua::TrahlRuntime;
use crate::rpc::{JobMsg, JobStatus, JobStatusMsg};

struct RunnerMessage {
    status_tx: mpsc::Sender::<JobStatusMsg>,
    spec: JobMsg,
}

pub struct JobRunner {
    tx: mpsc::Sender<RunnerMessage>,
    rx: Option<mpsc::Receiver<RunnerMessage>>,
}

impl JobRunner {
    pub fn new() -> Self {  
        let (tx, rx) = mpsc::channel(32);
        Self { 
            tx,
            rx: Some(rx),
        }
    }

    pub fn run(mut self) -> (JobRunner, JoinHandle<()>) {
        let mut rx = self.rx
            .take()
            .expect("JobSpanwer::run() can be called only once");


        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mut runtime = TrahlRuntime::new(msg.spec.job_id)
                    .with_stdout(msg.status_tx.clone());
                runtime = runtime.with_vars(msg.spec.vars.clone());

                match runtime.build() {
                    Ok(lua) => {
                        let job = Job::new(
                            msg.spec,
                            lua,
                            msg.status_tx.clone(),
                        );
                        task::spawn(job.run());
                    },
                    Err(e) => {
                        let _ = msg.status_tx.send(JobStatusMsg {
                            job_id: msg.spec.job_id,
                            status: JobStatus::Error {
                                descr: format!("Failed to create lua context: {}", e),
                            },
                        });
                    }
                }
            }
        });

        (self, handle)
    }
    
    pub async fn spawn_job(&self, spec: JobMsg, status_tx: mpsc::Sender<JobStatusMsg>) {
        let msg = RunnerMessage { 
            spec,
            status_tx,
        };

        let _ = self.tx.send(msg).await.map_err(|_| "Runner closed");
    }
}

struct Job {
    spec: JobMsg,
    luactx: Lua,
    status_tx: mpsc::Sender<JobStatusMsg>,
}

impl Job {
    fn new(spec: JobMsg,
            luactx: Lua,
            status_tx: mpsc::Sender<JobStatusMsg>,
        ) -> Self {
        Job {
            spec,
            luactx,
            status_tx,
        }
    }

    async fn run(self) {
        let res = self.luactx
            .load(self.spec.script)
            .exec_async()
            .await;

        match res {
            Ok(_) => {
                info!("Job {} finished", self.spec.job_id);

                if let Err(e) = self.status_tx.send(JobStatusMsg {
                    job_id: self.spec.job_id,
                    status: JobStatus::Done,
                }).await {
                    error!("Error while sending message: {}", e);
                }
            }
            Err(e) => {
                error!("Job {} failed", self.spec.job_id);
                if let Err(e) = self.status_tx.send(JobStatusMsg {
                    job_id: self.spec.job_id,
                    status: JobStatus::Error{ descr: e.to_string() },
                }).await {
                    error!("Error while sending message: {}", e);
                }
            }
        }
    }
}
