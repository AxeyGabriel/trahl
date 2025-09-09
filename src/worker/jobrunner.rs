use tokio::sync::{mpsc, oneshot};
use tokio::task::{self, JoinHandle};
use mlua::Lua;
use std::collections::HashMap;

use crate::lua;

pub type JobResult = Result<(), String>;

pub struct JobSpec {
    pub script: String,
    pub vars: Option<HashMap<String, String>>
}

pub struct Job {
    spec: JobSpec,
    luactx: Lua,
    result_tx: oneshot::Sender<JobResult>,
}

impl Job {
    fn new(spec: JobSpec, luactx: Lua, result_tx: oneshot::Sender<JobResult>) -> Self {
        Job {
            spec,
            luactx,
            result_tx,
        }
    }

    async fn run(self) {
        let res = self.luactx
            .load(self.spec.script)
            .exec_async()
            .await;

        match res {
            Ok(_) => {
                let _ = self.result_tx.send(Ok(()));
            }
            Err(e) => {
                let _ = self.result_tx.send(Err(e.to_string()));
            }
        }
    }
}

struct RunnerMessage {
    spec: JobSpec,
    result_tx: oneshot::Sender<JobResult>,
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

    pub async fn spawn_job(&self, spec: JobSpec) -> JobResult {
        let (result_tx, result_rx) = oneshot::channel();
        let msg = RunnerMessage { spec, result_tx };
        self.tx.send(msg).await.map_err(|_| "Runner closed")?;
        result_rx.await.map_err(|_| "Job task cancelled")?
    }

    pub fn run(mut self) -> (JobRunner, JoinHandle<()>) {
        let mut rx = self.rx
            .take()
            .expect("JobSpanwer::run() can be called only once");

        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match lua::create_lua_context(msg.spec.vars.clone()) {
                    Ok(lua) => {
                        let job = Job::new(msg.spec, lua, msg.result_tx);
                        task::spawn(job.run());
                    }
                    Err(e) => {
                        let _ = msg.result_tx.send(Err(
                            format!("Failed to create lua context: {}", e)
                        ));
                    }
                }
            }
        });

        (self, handle)
    }
}
