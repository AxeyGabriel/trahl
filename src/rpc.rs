pub mod zmq_helper;

use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub enum Message {
    Hello(HelloMsg),
    HelloAck,
    ConfigUpdate(ConfigMsg),
    Ping,
    Pong,
    Job(JobMsg),
    JobAck(JobAckMsg),
    JobStatus(JobStatusMsg),
    JobDone(JobDoneMsg),
    FileTransferReq(FileRequestMsg),
    FileChunk(FileChunkMsg),
    FileTransferOk,
    FileTransferFail,
    Bye,
}

#[derive(Debug, Encode, Decode)]
pub struct HelloMsg {
    pub identifier: String,
    pub simultaneous_jobs: u8,
}

#[derive(Debug, Encode, Decode)]
pub struct ConfigMsg {
 
}

#[derive(Debug, Encode, Decode)]
pub struct JobMsg {

}

#[derive(Debug, Encode, Decode)]
pub struct JobAckMsg {

}

#[derive(Debug, Encode, Decode)]
pub struct JobStatusMsg {

}

#[derive(Debug, Encode, Decode)]
pub struct JobDoneMsg {

}

#[derive(Debug, Encode, Decode)]
pub struct FileRequestMsg {

}

#[derive(Debug, Encode, Decode)]
pub struct FileChunkMsg {

}

