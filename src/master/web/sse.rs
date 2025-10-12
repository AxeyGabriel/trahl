use axum::{
    response::{
        sse::{
            Event,
            Sse,
            KeepAlive,
        },
    },
};
use futures::Stream;
use async_stream::try_stream;
use std::time::Duration;
use std::convert::Infallible;

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

pub async fn test() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(try_stream! {
        let mut id: u64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let event = Event::default()
                .event("TestEvent")
                .id(id.to_string())
                .data(id.to_string());
            id += 1;
            yield event;
        }
    })
    .keep_alive(KeepAlive::default())
}
