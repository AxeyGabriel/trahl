use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::task::{self, JoinHandle};
use mlua::Lua;
use tracing::{info, error};

use crate::config::FsRemap;
use crate::lua::TrahlRuntime;
use crate::rpc::{JobMsg, JobStatus, JobStatusMsg};
use crate::utils;

struct RunnerMessage {
    status_tx: mpsc::Sender::<JobStatusMsg>,
    spec: JobMsg,
}

pub struct JobRunner {
    tx: mpsc::Sender<RunnerMessage>,
    rx: Option<mpsc::Receiver<RunnerMessage>>,
    tmpdir: PathBuf,
    fsremaps: Option<Vec<FsRemap>>,
}

impl JobRunner {
    pub fn new(tmpdir: PathBuf, fsremaps: Option<Vec<FsRemap>>) -> Self {  
        let (tx, rx) = mpsc::channel(32);
        Self { 
            tx,
            rx: Some(rx),
            tmpdir,
            fsremaps
        }
    }

    pub fn run(mut self) -> (JobRunner, JoinHandle<()>) {
        let mut rx = self.rx
            .take()
            .expect("JobSpanwer::run() can be called only once");

        let tmpdir_clone = self.tmpdir.clone();
        let remaps_clone = self.fsremaps.clone();

        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let tmpdir = match TempDir::new_in(tmpdir_clone.clone()) {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Job {} failed: {}", msg.spec.job_id, e);
                        if let Err(se) = msg.status_tx.send(JobStatusMsg {
                            job_id: msg.spec.job_id,
                            status: JobStatus::Error{ descr: e.to_string() },
                        }).await {
                            error!("Error while sending message: {}", se);
                        }
                        return
                    }
                };

                let mut runtime = TrahlRuntime::new(msg.spec.job_id)
                    .with_stdout(msg.status_tx.clone());
                let mut vars = msg.spec.vars.clone();
                vars.insert("CACHEDIR".to_string(), tmpdir.path().to_str().unwrap().to_string());

                let orig_src = Path::new(&msg.spec.file);
                let srcfile = utils::remap_to_worker(&orig_src, &remaps_clone);
                vars.insert("SRCFILE".to_string(), srcfile.to_string_lossy().to_string());

                let orig_dst = Path::new(&msg.spec.dst_dir);
                let dstdir = utils::remap_to_worker(&orig_dst, &remaps_clone);
                vars.insert("DSTDIR".to_string(), dstdir.to_string_lossy().to_string());
                
                let orig_libroot = Path::new(&msg.spec.library_root);
                let libroot = utils::remap_to_worker(&orig_libroot, &remaps_clone);

                runtime = runtime.with_vars(vars);

                match runtime.build() {
                    Ok(lua) => {
                        let job = Job::new(
                            msg.spec,
                            lua,
                            srcfile,
                            tmpdir,
                            dstdir,
                            libroot,
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
    _tmpdir: TempDir,
    dst_dir: PathBuf,
    library_root: PathBuf,
    status_tx: mpsc::Sender<JobStatusMsg>,
    original_file: PathBuf,
}

impl Job {
    fn new(spec: JobMsg,
            luactx: Lua,
            original_file: PathBuf,
            tmpdir: TempDir,
            dst_dir: PathBuf,
            library_root: PathBuf,
            status_tx: mpsc::Sender<JobStatusMsg>,
        ) -> Self {
        Job {
            spec,
            luactx,
            status_tx,
            _tmpdir: tmpdir,
            dst_dir,
            library_root,
            original_file,
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
                let mut result: Option<String> = None;
                match self.luactx.named_registry_value::<String>("output") {
                    Ok(file) => {
                        let mode = self.luactx.named_registry_value::<u8>("output_mode").unwrap();
                        /* MODE:
                         * 1 = PRESERVE DIR
                         * 2 = FLAT
                         * 3 = OVERWRITE
                         */

                        let file = Path::new(&file);
                        if !file.exists() {
                            let log = format!("File {} does not exist!", file.display());
                            error!("{}", log);
                            if let Err(se) = self.status_tx.send(JobStatusMsg {
                                job_id: self.spec.job_id,
                                status: JobStatus::Error { descr: log },
                            }).await {
                                error!("Error while sending message: {}", se);
                            } 
                            return;
                        }

                        if let Err(se) = self.status_tx.send(JobStatusMsg {
                            job_id: self.spec.job_id,
                            status: JobStatus::Copying,
                        }).await {
                            error!("Error while sending message: {}", se);
                            return;
                        }

                        match mode {
                            1 => {
                                let dst_path = match utils::copy_preserve_structure(
                                    self.original_file.as_path(),
                                    file,
                                    self.library_root.as_path(),
                                    self.dst_dir.as_path()).await
                                {
                                    Ok(d) => d,
                                    Err(e) => {
                                        if let Err(se) = self.status_tx.send(JobStatusMsg {
                                            job_id: self.spec.job_id,
                                            status: JobStatus::Error { descr: e.to_string() },
                                        }).await {
                                            error!("Error while sending message: {}", se);
                                        }
                                        return;
                                    }
                                };
                                result = Some(dst_path.to_string_lossy().to_string());
                            },
                            2 => {
                                let filename = file.file_name().unwrap();
                                let dst_path = Path::new(&self.dst_dir).join(filename);
                                tokio::fs::copy(file, &dst_path).await.unwrap();
                                result = Some(dst_path.to_string_lossy().to_string());
                            },
                            3 => {
                                tokio::fs::copy(file, self.original_file.clone()).await.unwrap();
                                result = Some(self.original_file.to_string_lossy().to_string());
                            },
                            _ => {
                                if let Err(se) = self.status_tx.send(JobStatusMsg {
                                    job_id: self.spec.job_id,
                                    status: JobStatus::Error { descr: "Unknown output mode".to_string() },
                                }).await {
                                    error!("Error while sending message: {}", se);
                                }
                                return;
                            }
                        }
                    },
                    _ => {},
                };

                if let Err(se) = self.status_tx.send(JobStatusMsg {
                    job_id: self.spec.job_id,
                    status: JobStatus::Done { file: result },
                }).await {
                    error!("Error while sending message: {}", se);
                }
            }
            Err(e) => {
                error!("Job {} failed: {}", self.spec.job_id, e);
                if let Err(se) = self.status_tx.send(JobStatusMsg {
                    job_id: self.spec.job_id,
                    status: JobStatus::Error { descr: e.to_string() },
                }).await {
                    error!("Error while sending message: {}", se);
                }
            }
        }
    }
}
