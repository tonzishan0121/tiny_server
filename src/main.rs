mod app_state;
mod chat_app;
mod server_core;

use app_state::AppState;
use chat_app::handlers::{chat_page, create_message, list_messages};
use server_core::config;
use server_core::router::{
    debug_error_handler, echo_handler, health_handler, home_handler, static_handler,
    static_index_handler,
};

fn main() {
    let server_config =
        config::load_server_config("config/server.yaml").expect("failed to load server config");
    let addr = format!("{}:{}", server_config.host, server_config.port);
    let app_state = AppState::new();
    let routes = routes![
        GET "/" => home_handler,
        GET "/health" => health_handler,
        GET "/index.html" => static_index_handler,
        GET "/static/" => static_handler [prefix],
        POST "/echo" => echo_handler,
        GET "/debug/500" => debug_error_handler,
        GET "/chat" => chat_page,
        GET "/api/messages" => list_messages,
        POST "/api/messages" => create_message,
    ];

    server_core::server::run(&addr, server_config, routes, app_state)
        .expect("failed to run server");
}
