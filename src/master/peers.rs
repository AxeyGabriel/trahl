use tracing::info;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::rpc::{
    Message,
    HelloMsg,
};

pub type PeerId = Vec<u8>;

pub type TxCoreDriverMsg = (PeerId, Message);
pub type RxCoreDriverMsg = Message;
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
    tx_to_manager: mpsc::Sender<TxCoreDriverMsg>,
    rx_from_manager: mpsc::Receiver<RxCoreDriverMsg>,
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
        tx_to_manager: mpsc::Sender<TxCoreDriverMsg>,
        rx_from_manager: mpsc::Receiver<RxCoreDriverMsg>,
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

    pub async fn receive_from_socket(&mut self) -> Option<RxSocketMsg> {
        let msg = self.rx_from_socket.recv().await;

        if msg.is_some() {
            self.last_seen = Instant::now();
        }

        msg
    }
    
    async fn send_to_driver(&self, msg: Message) {
        let _ = self.tx_to_manager
            .send((self.socket_id.clone(), msg))
            .await;
    }

    pub async fn receive_from_driver(&mut self) -> Option<RxCoreDriverMsg> {
        let msg = self.rx_from_manager.recv().await;

        msg
    }

    pub async fn run(mut self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            //self.send_to_socket(Message::Ping).await;
            if let Some(msg) = self.rx_from_manager.recv().await {
                info!("peer:rx_from_manager: {:#?}", msg);
            }

            // send ping to socket
            // wait for message from socket
            // if ping, send it to driver
        }
    }
}
