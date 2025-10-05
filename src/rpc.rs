pub mod zmq_helper;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum Message {
    Hello(WorkerInfo),              // Worker -> Master, with worker capabilities
    HelloAck,                       // Master -> Worker
    CancelJobs,                     // Master -> Worker, cancel all running/pending jobs
    Ping,                           // Master -> Worker
    Pong,                           // Worker -> Master
    Job(JobMsg),                    // Master -> Worker
    JobStatus(JobStatusMsg),        // Worker -> Master
/*
    FileTransfer(FileTransferMsg),  // Master -> Worker
    FileChunk(FileChunkMsg),        // Master -> Worker
    FileTransferStatus,             // Worker -> Master
*/
    Bye,
}

impl Message {
    pub fn hello(wi: WorkerInfo) -> Self {
        Self::Hello(wi)
    }
    
    pub fn ack() -> Self {
        Self::HelloAck
    }
    
    pub fn cancel_jobs() -> Self {
        Self::CancelJobs
    }
    
    pub fn job_status(jsm: JobStatusMsg) -> Self {
        Self::JobStatus(jsm)
    }
    
    pub fn job(jm: JobMsg) -> Self {
        Self::Job(jm)
    }
    
    pub fn ping() -> Self {
        Self::Ping
    }
    
    pub fn pong() -> Self {
        Self::Pong
    }
    
    pub fn bye() -> Self {
        Self::Bye
    }
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct WorkerInfo {
    pub identifier: String,
    pub simultaneous_jobs: u8,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobMsg {
    pub job_id: u128,
    pub script: String,
    pub vars: HashMap<String, String>,
    pub file: String,
    pub library_root: String,
    pub dst_dir: String,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobStatusMsg {
    pub timestamp: u64,
    pub job_id: u128,
    pub status: JobStatus,
}

impl JobStatusMsg {
    fn new(job_id: u128, status: JobStatus) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp,
            job_id,
            status
        }
    }
    
    pub fn job_ack(job_id: u128) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Ack)
    }
    
    pub fn job_declined(job_id: u128, reason: String) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Declined(reason))
    }
    
    pub fn job_progress(job_id: u128, tp: TranscodeProgress) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Progress(tp))
    }
    
    pub fn job_copying(job_id: u128) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Copying)
    }
    
    pub fn job_milestone(job_id: u128, m: String) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Milestone(m))
    }
    
    pub fn job_log(job_id: u128, l: String) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Log(l))
    }
    
    pub fn job_error(job_id: u128, e: String) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Error(e))
    }
    
    pub fn job_done(job_id: u128, file: Option<String>) -> Self {
        JobStatusMsg::new(job_id, JobStatus::Done {file})
    }
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum JobStatus {
    Ack,
    Declined(String),
    Progress(TranscodeProgress),
    Copying,
    Milestone(String),
    Log(String),
    Error(String),
    Done {
        file: Option<String>,
    },
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct TranscodeProgress {
    pub frame: Option<u64>,
    pub fps: Option<u64>,
    pub cur_time: Option<Duration>,
    pub percentage: Option<f64>,
    pub eta: Option<Duration>,
    pub bitrate: Option<String>,
    pub speed: Option<f64>,
}

/*
#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct FileTransferMsg {
    filename: String,
    hash: String,
    size: u64,
    chunk_size: u64,
    chunk_total: u64,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct FileChunkMsg {
    bytes: Vec<u8>,
    chunk: u64,
}
*/
