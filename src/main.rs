mod app_state;
mod chat_app;
mod concurrency;
mod config;
#[allow(dead_code, unused_imports)]
mod database;
mod http;
mod router;
mod server;

use app_state::AppState;
use chat_app::handlers::{
    api_health, api_meta, chat_page, create_message, create_room, delete_room, list_messages,
    list_rooms, rename_room,
};
use router::{
    debug_error_handler, echo_handler, health_handler, home_handler, static_handler,
    static_index_handler,
};

fn main() {
    let server_config_path = std::env::var("TINY_SERVER_CONFIG")
        .unwrap_or_else(|_| "config/server.yaml".to_string());
    let database_config_path = std::env::var("TINY_DATABASE_CONFIG")
        .unwrap_or_else(|_| "config/database.yaml".to_string());

    let server_config =
        config::load_server_config(&server_config_path).expect("failed to load server config");
    let addr = format!("{}:{}", server_config.host, server_config.port);
    let app_state =
        AppState::new(&database_config_path).expect("failed to initialize app state");
    let routes = routes![
        GET "/" => home_handler,
        GET "/health" => health_handler,
        GET "/index.html" => static_index_handler,
        GET "/static/" => static_handler [prefix],
        POST "/echo" => echo_handler,
        GET "/debug/500" => debug_error_handler,
        GET "/chat" => chat_page,
        GET "/api/health" => api_health,
        GET "/api/meta" => api_meta,
        GET "/api/rooms" => list_rooms,
        POST "/api/rooms" => create_room,
        POST "/api/rooms/rename" => rename_room,
        POST "/api/rooms/delete" => delete_room,
        GET "/api/messages" => list_messages,
        POST "/api/messages" => create_message,
    ];

    server::run(&addr, server_config, routes, app_state).expect("failed to run server");
}
