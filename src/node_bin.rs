mod node;
use log::{error, info};
use std::time::{Duration, Instant};

use crate::node::{config::NodeConfigFile, context::NodeContext};
use shared_lib::{config, file::SystemFileAccessor, logger};

#[tokio::main]
async fn main() {
    // setup logger (defaults the log level to info)
    logger::setup_logger();

    let config_path = config::discover_config_path().expect("Failed to discover config path");
    info!("Using configuration from: {}", config_path);

    let config = config::load_config::<NodeConfigFile>(config_path).expect("Failed to load config");
    let accessor = SystemFileAccessor;
    let mut context = NodeContext::init(&config, Box::new(accessor))
        .await
        .expect("Failed to initialize context");

    if let Err(err) = context.push_metrics().await {
        error!("Failed to push metrics: {}", err);
    }
    if let Err(err) = context.pull_wireguard().await {
        error!("Failed to pull wireguard: {}", err);
    }

    let interval_pull = config.node.pull_interval as u64;
    let interval_metrics = config.node.metrics_interval as u64;

    let mut next_pull = Instant::now() + Duration::from_secs(interval_pull);
    let mut next_metrics_push = Instant::now() + Duration::from_secs(interval_metrics);

    loop {
        let now = Instant::now();

        if now >= next_pull {
            next_pull += Duration::from_secs(interval_pull);
            if let Err(err) = context.pull_wireguard().await {
                error!("Failed to pull wireguard: {}", err);
            }
        }

        if now >= next_metrics_push {
            next_metrics_push += Duration::from_secs(interval_metrics);
            if let Err(err) = context.push_metrics().await {
                error!("Failed to push metrics: {}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
