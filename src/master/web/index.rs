use maud::{html, Markup, DOCTYPE};

pub fn index() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Trahl" }
                script src="/static/htmx.min.js" {}
                script src="/static/htmx-ext-sse.min.js" {}
                link rel="stylesheet" href="/static/style.css" {}
            }
            body {
                (taskbar())
                script src="/static/libwm.js" {}
            }
        }
    }
}

fn taskbar() -> Markup {
    html! {
        div.taskbar {
            div.start-button { "Start" }
            div.start-menu {
                ul.start-menu-list {
                    li.start-menu-item data-window="window-statistics" { "Statistics" }
                    li.start-menu-item data-window="window-queue" { "Queue" }
                    li.start-menu-item data-window="window-activity" { "Activity" }
                    li.start-menu-item data-window="window-control" { "Control" }
                    li.start-menu-item data-window="window-syslog" { "System Logs" }
                }
            }
            div.taskbar-items { }
            div.taskbar-clock
                hx-ext="sse"
                sse-connect="/sse/clock"
                sse-swap="ClockEvent" #clock {}
        }
    }
}
