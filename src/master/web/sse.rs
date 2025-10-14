use axum::{
    extract::State,
    response::sse::{
        Event, KeepAlive, Sse
    }
};
use futures::Stream;
use async_stream::try_stream;
use std::time::Duration;
use std::convert::Infallible;

use crate::master::{manager::ManagerEvent, web::AppState};

pub async fn clock() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(try_stream! {
        let mut id: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = chrono::Local::now().format("%H:%M:%S").to_string();
            let event = Event::default()
                .event("ClockEvent")
                .id(id.to_string())
                .data(now);
            id += 1;
            yield event;
        }
    })
    .keep_alive(KeepAlive::default())
}

pub async fn test(
    State(state): State<AppState>
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(try_stream! {
        let mut rx = state.broadcast.subscribe();
        while let Ok(msg) = rx.recv().await {
            match msg {
                ManagerEvent::PeerList { wi } => {
                    let event = Event::default()
                        .event("PeerList")
                        .data(wi.identifier);
                    yield event;
                },
                _ => {}
            }
        }
    })
    .keep_alive(KeepAlive::default())
}
