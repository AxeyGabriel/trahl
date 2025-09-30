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
    pub vars: HashMap<String, String>
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
    Log {
        line: String,
    },
    Error {
        descr: String,
    },
    Done,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct TranscodeProgress {
    frame: Option<u64>,
    fps: Option<f64>,
    cur_time: Option<Duration>,
    percentage: Option<f64>,
    eta: Option<Duration>,
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
