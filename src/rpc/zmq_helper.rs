use anyhow::Result;
use zeromq::{SocketRecv, SocketSend, ZmqMessage};

use super::*;

pub async fn recv_msg<S>(socket: &mut S, router: bool) -> Result<(Option<Vec<u8>>, Message)> 
where
    S: SocketRecv
{
    let frames = socket.recv().await?.into_vec();
    
    let (client_id, payload) = if router {
        (Some(frames[0].to_vec()), &frames[1])
    } else {
        (None, &frames[0])
    };

    let msg: Message = bincode::decode_from_slice(payload, bincode::config::standard())?.0;

    Ok((client_id, msg))
}

pub async fn send_msg<S>(
    socket: &mut S,
    client_id: Option<&[u8]>,
    msg: &Message,
) -> Result<()> 
where
    S: SocketSend
{
    let bytes = bincode::encode_to_vec(msg, bincode::config::standard())?;
    let mut msg = ZmqMessage::from(bytes);

    if let Some(client_id) = client_id {
        msg.push_front(client_id.to_owned().into());
    }

    socket.send(msg).await?;

    Ok(())
}
