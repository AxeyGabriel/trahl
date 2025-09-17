use tracing::{info, warn};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::rpc::{
    Message,
    HelloMsg,
};

pub type PeerId = Vec<u8>;

pub type TxManagerMsg = (PeerId, Message);
pub type RxManagerMsg = Message;
pub type TxSocketMsg = (PeerId, Message);
pub type RxSocketMsg = Message;

#[derive(Debug)]
pub struct Peer {
    socket_id: PeerId,
    params: HelloMsg,
    last_seen: Instant,
    state: State,
    tx_to_socket: mpsc::Sender<TxSocketMsg>,
    rx_from_socket: mpsc::Receiver<RxSocketMsg>,
    tx_to_manager: mpsc::Sender<TxManagerMsg>,
    rx_from_manager: mpsc::Receiver<RxManagerMsg>,
}

#[derive(Debug)]
enum State {
    NotReady,
    Configured,
}

impl Peer {
    pub fn new(
        hello: HelloMsg,
        socket_id: PeerId,
        tx_to_socket: mpsc::Sender<TxSocketMsg>,
        rx_from_socket: mpsc::Receiver<RxSocketMsg>,
        tx_to_manager: mpsc::Sender<TxManagerMsg>,
        rx_from_manager: mpsc::Receiver<RxManagerMsg>,
    ) -> Self {
        Peer {
            last_seen: Instant::now(),
            state: State::NotReady,
            socket_id,
            params: hello,
            tx_to_socket,
            rx_from_socket,
            tx_to_manager,
            rx_from_manager,
        }
    }

    async fn send_to_socket(&self, msg: Message) {
        let _ = self.tx_to_socket
            .send((self.socket_id.clone(), msg))
            .await;
    }
    
    async fn send_to_manager(&self, msg: Message) {
        let _ = self.tx_to_manager
            .send((self.socket_id.clone(), msg))
            .await;
    }

    pub async fn run(mut self) {
        let mut keepalive_timer = interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                Some(msg) = self.rx_from_manager.recv() => {
                    self.message_from_manager(msg).await;
                },
                Some(msg) = self.rx_from_socket.recv() => {
                    self.last_seen = Instant::now();
                    self.message_from_socket(msg).await;
                },
                _ = keepalive_timer.tick() => {
                    self.send_to_socket(Message::Ping).await;

                    if self.last_seen.elapsed() >= Duration::from_secs(5) {
                        // Socket timed out, abort
                        warn!("Peer {} timed out", self.params.identifier);
                        self.send_to_socket(Message::Bye).await;
                    }
                }
            }
        }
    }

    async fn message_from_manager(&mut self, msg: RxManagerMsg) {
        //todo: implement file transfers
        self.send_to_socket(msg).await;
    }
    
    async fn message_from_socket(&mut self, msg: RxSocketMsg) {
        self.send_to_manager(msg).await;
    }
}
