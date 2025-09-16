use tracing::info;
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
    handshake_state: Handshake,
    tx_to_socket: mpsc::Sender<TxSocketMsg>,
    rx_from_socket: mpsc::Receiver<RxSocketMsg>,
    tx_to_manager: mpsc::Sender<TxManagerMsg>,
    rx_from_manager: mpsc::Receiver<RxManagerMsg>,
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
        tx_to_socket: mpsc::Sender<TxSocketMsg>,
        rx_from_socket: mpsc::Receiver<RxSocketMsg>,
        tx_to_manager: mpsc::Sender<TxManagerMsg>,
        rx_from_manager: mpsc::Receiver<RxManagerMsg>,
    ) -> Self {
        Peer {
            last_seen: Instant::now(),
            handshake_state: Handshake::Disconnected,
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
                    info!("peer:rx_from_manager: {:#?}", msg);
                },
                Some(msg) = self.rx_from_socket.recv() => {
                    self.last_seen = Instant::now();
                    info!("peer:rx_from_socket: {:#?}", msg);
                },
                _ = keepalive_timer.tick() => {
                    self.send_to_socket(Message::Ping).await;
                }
            }

            // send ping to socket
            // wait for message from socket
            // if ping, send it to manager
        }
    }
}
