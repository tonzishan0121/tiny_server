mod config;
mod http;
mod router;
mod server;

fn main() {
    let server_config =
        config::load_server_config("config/server.yaml").expect("failed to load server config");
    let routes = router::load_routes("config/routes.yaml").expect("failed to load routes config");
    let addr = format!("{}:{}", server_config.host, server_config.port);
    server::run(&addr, &routes).expect("failed to run server");
}
