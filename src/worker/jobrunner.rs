use std::path::{Path, PathBuf};

use anyhow::anyhow;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::task::{self, JoinHandle};
use tracing::{info, error};

use crate::config::FsRemap;
use crate::lua::{TrahlRuntime, TrahlRuntimeBuilder};
use crate::rpc::{JobMsg, JobStatusMsg};
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
        let (tx, rx) = mpsc::channel(8);
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
                let job_id_clone = msg.spec.job_id;
                let job = Job::new(
                    msg.spec,
                    tmpdir_clone.clone(),
                    remaps_clone.clone(),
                    msg.status_tx.clone(),
                ).await;

                match job {
                    Ok(job) => {
                        let _ = msg.status_tx.send(JobStatusMsg::job_ack(job_id_clone))
                            .await
                            .inspect_err(|e| { 
                                error!("Error sending message: {}", e)
                            });
                        task::spawn(job.run());
                    },
                    Err(e) => {
                        let err_str = format!("Job {} failed: {}", job_id_clone, e);
                        error!(err_str);
                        let _ = msg.status_tx.send(
                                JobStatusMsg::job_declined(job_id_clone, e.to_string())
                            )
                            .await
                            .inspect_err(|e| { error!("Error sending message: {}", e) });
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

        let _ = self.tx.send(msg).await.inspect_err(|_| error!("Runner closed"));
    }
}

struct Job {
    spec: JobMsg,
    _tmpdir: TempDir,
    status_tx: mpsc::Sender<JobStatusMsg>,
    _runtime: TrahlRuntime,
    remaps: Option<Vec<FsRemap>>,
}

impl Job {
    pub async fn new(spec: JobMsg,
            tmpdir_path: PathBuf,
            remaps: Option<Vec<FsRemap>>,
            status_tx: mpsc::Sender<JobStatusMsg>,
        ) -> anyhow::Result<Self> {
        let tmpdir = match TempDir::new_in(tmpdir_path.clone()) {
            Ok(t) => t,
            Err(e) => {
                let err_str = format!("Job {} failed: {}", spec.job_id, e);
                error!(err_str);
                status_tx.send(
                        JobStatusMsg::job_error(spec.job_id, err_str.clone())
                    )
                    .await
                    .map_err(|e| anyhow!("status_tx failed: {}", e))?;
                return Err(anyhow!(err_str));
            }
        };

        let mut vars = spec.vars.clone();
        vars.insert("CACHEDIR".to_string(), tmpdir.path().to_str().unwrap().to_string());

        let orig_src = Path::new(&spec.file);
        let srcfile = utils::remap_to_worker(&orig_src, &remaps);
        vars.insert("SRCFILE".to_string(), srcfile.to_string_lossy().to_string());

        let orig_dst = Path::new(&spec.dst_dir);
        let dstdir = utils::remap_to_worker(&orig_dst, &remaps);
        vars.insert("DSTDIR".to_string(), dstdir.to_string_lossy().to_string());
        
        let orig_libroot = Path::new(&spec.library_root);
        let libroot = utils::remap_to_worker(&orig_libroot, &remaps);
        vars.insert("LIBRARYROOT".to_string(), libroot.to_string_lossy().to_string());

        let runtime = TrahlRuntimeBuilder::new(
            spec.job_id,
            status_tx.clone(),
            spec.script.clone())
            .add_vars(vars)
            .build()
            .unwrap();

        Ok(Job {
            spec,
            _runtime: runtime,
            status_tx,
            _tmpdir: tmpdir,
            remaps
        })
    }

    async fn run(self) {
        let res = self._runtime.exec().await;
        match res {
            Ok(_) => {
                info!("Job {} finished", self.spec.job_id);
                let mut result: Option<String> = None;
                match self._runtime.get_output() {
                    Ok(file) => {
                        let mode = self._runtime.get_output_mode().unwrap();
                        /* MODE:
                         * 1 = PRESERVE DIR
                         * 2 = FLAT
                         * 3 = OVERWRITE
                         */

                        let file = Path::new(&file);
                        if !file.exists() {
                            let log = format!("File {} does not exist!", file.display());
                            error!("{}", log);
                            let _ = self.status_tx.send(
                                    JobStatusMsg::job_error(self.spec.job_id, log)
                                ).await
                                .inspect_err(|e| { error!("Error sending message: {}", e) });
                            return;
                        }

                        let _ = self.status_tx.send(
                                JobStatusMsg::job_copying(self.spec.job_id)
                            ).await
                            .inspect_err(|e| { error!("Error sending message: {}", e) });

                        let original_file_remapped = utils::remap_to_worker(Path::new(&self.spec.file), &self.remaps);
                        let library_root_remapped = utils::remap_to_worker(Path::new(&self.spec.library_root), &self.remaps);
                        let destination_dir_remapped = utils::remap_to_worker(Path::new(&self.spec.dst_dir), &self.remaps);

                        match mode {
                            1 => {
                                let dst_path = match utils::copy_preserve_structure(
                                    original_file_remapped.as_path(),
                                    file,
                                    library_root_remapped.as_path(),
                                    destination_dir_remapped.as_path()).await
                                {
                                    Ok(d) => d,
                                    Err(e) => {
                                        let _ = self.status_tx.send(
                                                JobStatusMsg::job_error(self.spec.job_id, e.to_string())
                                            ).await
                                            .inspect_err(|e| { error!("Error sending message: {}", e) });
                                        return;
                                    }
                                };
                                result = Some(dst_path.to_string_lossy().to_string());
                            },
                            2 => {
                                let filename = file.file_name().unwrap();
                                let dst_path = Path::new(&destination_dir_remapped).join(filename);
                                tokio::fs::copy(file, &dst_path).await.unwrap();
                                result = Some(dst_path.to_string_lossy().to_string());
                            },
                            3 => {
                                tokio::fs::copy(file, original_file_remapped.clone()).await.unwrap();
                                result = Some(self.spec.file);
                            },
                            _ => {
                                let _ = self.status_tx.send(
                                        JobStatusMsg::job_error(self.spec.job_id, "Unknown output mode".to_string())
                                    ).await
                                    .inspect_err(|e| { error!("Error sending message: {}", e) });
                                return;
                            }
                        }
                    },
                    _ => {},
                };

                let _ = self.status_tx.send(
                        JobStatusMsg::job_done(self.spec.job_id, result)
                    ).await
                    .inspect_err(|e| { error!("Error sending message: {}", e) });
            }
            Err(e) => {
                error!("Job {} failed: {}", self.spec.job_id, e);
                let _ = self.status_tx.send(
                        JobStatusMsg::job_error(self.spec.job_id, e.to_string())
                    ).await
                    .inspect_err(|e| { error!("Error sending message: {}", e) });
            }
        }
    }
}
