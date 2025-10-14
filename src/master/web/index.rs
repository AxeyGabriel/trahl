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
                div id="modal-overlay" {
				    (modal_conn_lost())
                }
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
                id="clock"
                hx-ext="sse"
                sse-connect="/sse/clock"
                sse-swap="ClockEvent" #clock {}
        }
    }
}

fn modal_conn_lost() -> Markup {
    html! {
		div.mdi-window.modal id="modal-lostconn"
            style="left: 50%; top: 50%; width: 400px; height: 200px; z-index: 9001; transform: translate(-50%, -50%); display: none;" {
            
            div.title-bar.no-move {
                span.title-text { "Disconnected" }
            }

            div.window-content {
                div style="padding: 20px; text-align: center;" {        
                    div style="font-size: 14px; font-weight: bold; margin-bottom: 16px;" {
                        "Connection lost to the server"
                    }
                    div style="font-size: 11px; color: #666; margin-bottom: 20px;" {
                        "Please reload the page or try again later"
                    }
                    div style="display: flex; gap: 8px; justify-content: center;" {
                        button.button onclick="location.reload()" { "Ok" }
                    }
                }
            }
		}
    }
}
