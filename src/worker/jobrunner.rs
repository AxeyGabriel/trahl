use tokio::sync::{mpsc, oneshot};
use tokio::task::{self, JoinHandle};
use mlua::Lua;
use std::collections::HashMap;

use crate::lua::{self, TrahlRuntime};

pub type JobResult = Result<(), String>;

pub struct JobRunner {
    tx: mpsc::Sender<RunnerMessage>,
    rx: Option<mpsc::Receiver<RunnerMessage>>,
}

pub struct JobSpec {
    pub script: String,
    pub vars: Option<HashMap<String, String>>
}

#[derive(Debug)]
pub struct JobHandle {
    pub result_rx: oneshot::Receiver<JobResult>,
    pub output_rx: mpsc::Receiver<String>,
}

struct RunnerMessage {
    spec: JobSpec,
    result_tx: oneshot::Sender<JobResult>,
    output_tx: mpsc::Sender<String>,
}

pub struct Job {
    spec: JobSpec,
    luactx: Lua,
    result_tx: oneshot::Sender<JobResult>,
}

impl JobHandle {
    pub async fn await_result(self) -> JobResult {
        self.result_rx
            .await
            .unwrap_or_else(|_| Err("Job cancelled".into()))
    }

    pub fn stdout_stream(&mut self) -> &mut mpsc::Receiver<String> {
        &mut self.output_rx
    }
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
                let mut runtime = TrahlRuntime::new()
                    .with_stdout(msg.output_tx);
                if let Some(vars) = &msg.spec.vars {
                    runtime = runtime.with_vars(vars.clone());
                };

                match runtime.build() {
                    Ok(lua) => {
                        let job = Job::new(
                            msg.spec,
                            lua,
                            msg.result_tx,
                        );
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
    
    pub async fn spawn_job(&self, spec: JobSpec) -> JobHandle {
        let (result_tx, result_rx) = oneshot::channel();
        let (output_tx, output_rx) = mpsc::channel(32);

        let msg = RunnerMessage { 
            spec,
            result_tx,
            output_tx,
        };

        let _ = self.tx.send(msg).await.map_err(|_| "Runner closed");

        JobHandle {
            result_rx,
            output_rx,
        }
    }
}

impl Job {
    fn new(spec: JobSpec,
            luactx: Lua,
            result_tx: oneshot::Sender<JobResult>,
        ) -> Self {
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

