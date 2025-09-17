pub mod zmq_helper;

use std::path::PathBuf;
use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub enum Message {
    Hello(HelloMsg),
    HelloAck,
    Config(ConfigMsg),
    Ping,
    Pong,
    Job(JobMsg),
    JobAck(JobAckMsg),
    JobStatus(JobStatusMsg),
    JobDone(JobDoneMsg),
    FileTransfer(FileTransferMsg),
    FileChunk(FileChunkMsg),
    FileTransferOk,
    FileTransferError,
    Bye,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct HelloMsg {
    pub identifier: String,
    pub simultaneous_jobs: u8,
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct ConfigMsg {
 
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobMsg {

}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobAckMsg {

}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobStatusMsg {

}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct JobDoneMsg {

}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct FileTransferMsg {
}

#[derive(Debug, Encode, Decode, Clone, PartialEq)]
pub struct FileChunkMsg {
}

