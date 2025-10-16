use axum::{
    extract::State,
    response::sse::{
        Event, KeepAlive, Sse
    }
};
use futures::Stream;
use async_stream::try_stream;
use maud::{html, Markup};
use std::time::Duration;
use std::convert::Infallible;

use crate::master::web::AppState;
use crate::master::manager::events::*;

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

pub async fn manager_events(
    State(state): State<AppState>
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(try_stream! {
        let mut rx = state.broadcast.subscribe();
        while let Ok(msg) = rx.recv().await {
            match msg {
                ManagerEvent::JobQueue(jqes) => {
                    let data = queue_rows(jqes).into_string();
                    let event = Event::default()
                        .event("JobQueue")
                        .data(data);
                    yield event;
                },
                _ => {}
            }
        }
    })
    .keep_alive(KeepAlive::default())
}

fn queue_rows(jqes: Vec<JobQueueEntry>) -> Markup {
    html! {
        @for jqe in jqes {
            tr {
                td { (jqe.file) }
                td { (jqe.library) }
                @if jqe.status == "PROCESSING" {
                    td { span.status-badge.status-processing { "PROCESSING" } }
                } @else if jqe.status == "QUEUED" {
                    td { span.status-badge.status-queued { "QUEUED" } }
                }
                td { (jqe.worker) }
                td { (jqe.milestone) }
                @if jqe.progress == "-" {
                    td { "-" }
                } @else {
                    td {
                        div.progress-bar {
                            div.progress-fill style=(format!("width: {};", jqe.progress)) {}
                        }
                        span { (jqe.progress) }
                    }
                }
                td { (jqe.eta) }
            }
        }
    }
}
