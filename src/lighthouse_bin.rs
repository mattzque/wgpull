mod lighthouse;
use std::{net::SocketAddr, sync::Arc};

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
use shared_lib::{
    command::CommandExecutor,
    file::FileAccessor,
    time::{CurrentTime, SystemCurrentTime},
};
use shared_lib::{command::SystemCommandExecutor, file::SystemFileAccessor};

use crate::lighthouse::config::LighthouseConfigFile;
use shared_lib::{config, logger};

async fn make_router(
    config: LighthouseConfig,
    time: Arc<dyn CurrentTime + Send + Sync>,
    file_accessor: Arc<dyn FileAccessor + Send + Sync>,
    executor: Arc<dyn CommandExecutor + Send + Sync>,
) -> anyhow::Result<Router> {
    // create the lighthouse context to share across handlers
    let lighthouse = LighthouseContext::init(config, time, file_accessor, executor).await?;

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

    let app = make_router(
        config.lighthouse,
        Arc::new(SystemCurrentTime),
        Arc::new(SystemFileAccessor),
        Arc::new(SystemCommandExecutor),
    )
    .await
    .expect("Unable to create router!");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
