mod lighthouse;
use std::net::SocketAddr;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use lighthouse::context::LighthouseContext;
use lighthouse::handler;
use lighthouse::{
    config::LighthouseConfig, context::LighthouseContextProvider, handler::lighthouse_keys_layer,
};
use log::info;
use shared_lib::file::SystemFileAccessor;
use shared_lib::time::CurrentSystemTime;

use crate::lighthouse::config::LighthouseConfigFile;
use shared_lib::{config, logger};

async fn make_router(config: LighthouseConfig) -> anyhow::Result<Router> {
    // system time provider, uses SystemTime for telling the time
    let time = CurrentSystemTime;

    let file_accessor = SystemFileAccessor;

    // create the lighthouse context to share across handlers
    let lighthouse =
        LighthouseContext::init(config, Box::new(time), Box::new(file_accessor)).await?;

    // let state = Arc::new(lighthouse);
    let state = LighthouseContextProvider::new(lighthouse);

    let verify_keys_middleware =
        middleware::from_fn_with_state(state.clone(), lighthouse_keys_layer);

    let app = Router::new()
        .route(
            "/api/v1/pull",
            post(handler::post_pull_handler).layer(verify_keys_middleware.clone()),
        )
        .route(
            "/api/v1/metrics",
            post(handler::post_metrics_handler).layer(verify_keys_middleware),
        )
        .route("/metrics", get(handler::get_metrics_handler))
        .with_state(state);

    Ok(app)
}

#[tokio::main]
async fn main() {
    // setup logger (defaults the log level to info)
    logger::setup_logger();

    let config_path = config::discover_config_path().expect("Failed to discover config path");
    info!("Using configuration from: {}", config_path);

    let config =
        config::load_config::<LighthouseConfigFile>(config_path).expect("Failed to load config");
    let addr = config.lighthouse.get_listen_addr();

    info!("Lighthouse listening on: {}", addr);
    let addr = addr
        .parse::<SocketAddr>()
        .expect("Invalid bindhost/port for lighthouse!");

    let app = make_router(config.lighthouse)
        .await
        .expect("Unable to create router!");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
