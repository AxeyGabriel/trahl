pub mod zmq_helper;

use std::time::Duration;
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
    pub job_id: u128,
    pub status: JobStatus,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum JobStatus {
    Sent,
    Ack,
    Progress(TranscodeProgress),
    Copying,
    Log {
        line: String,
    },
    Error {
        descr: String,
    },
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
