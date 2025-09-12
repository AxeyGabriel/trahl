use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use tokio::sync::mpsc;

use crate::rpc::{
    Message,
    HelloMsg,
};

pub type PeerId = Vec<u8>;

#[derive(Debug)]
pub struct Peer {
    socket_id: PeerId,
    params: HelloMsg,
    last_seen: Instant,
    handshake_state: Handshake,
    tx: mpsc::Sender<(PeerId, Message)>,
    rx: mpsc::Receiver<Message>,
}

#[derive(Debug)]
enum Handshake {
    Disconnected,
    Discovered,
    ConfigUpdateSent,
    Ready,
}

impl Peer {
    pub fn new(
        hello: HelloMsg,
        socket_id: PeerId,
        tx: mpsc::Sender<(PeerId, Message)>,
        rx: mpsc::Receiver<Message>,
    ) -> Self {
        Peer {
            last_seen: Instant::now(),
            handshake_state: Handshake::Disconnected,
            socket_id,
            params: hello,
            tx,
            rx,
        }
    }

    async fn send(&self, msg: Message) {
        let _ = self.tx
            .send((self.socket_id.clone(), msg))
            .await;
    }

    pub async fn receive(&mut self) -> Option<Message> {
        let msg = self.rx.recv().await;

        if msg.is_some() {
            self.last_seen = Instant::now();
        }

        msg
    }

    pub fn get_params(&self) -> &HelloMsg {
        &self.params
    }

    pub fn get_last_seen(&self) -> &Instant {
        &self.last_seen
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
    }
}
