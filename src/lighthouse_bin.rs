mod lighthouse;
use gotham::anyhow;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::{single_middleware, single_pipeline};
use gotham::prelude::DefineSingleRoute;
use gotham::prelude::DrawRoutes;
use gotham::router::{build_router, Router};
use lighthouse::config::LighthouseConfig;
use lighthouse::context::LighthouseContext;
use lighthouse::handler;
use log::info;
use shared_lib::time::CurrentSystemTime;

use crate::lighthouse::config::LighthouseConfigFile;
use shared_lib::{logger, config};

fn router(config: LighthouseConfig) -> anyhow::Result<Router> {
    // system time provider, uses SystemTime
    let time = CurrentSystemTime::default();

    // create the lighthouse context to share across handlers
    let lighthouse = LighthouseContext::init(config, &time)?;

    // create our state middleware to share the context
    let middleware = StateMiddleware::new(lighthouse);

    // create a middleware pipeline from our middleware
    let pipeline = single_middleware(middleware);

    // construct a basic chain from our pipeline
    let (chain, pipelines) = single_pipeline(pipeline);

    // build a router with the chain & pipeline
    Ok(build_router(chain, pipelines, |route| {
        route.post("/api/v1/pull").to(handler::post_pull_handler);
        route.post("/api/v1/metrics").to(handler::post_metrics_handler);
        route.get("/metrics").to(handler::get_metrics_handler);
    }))
}

pub fn main() -> anyhow::Result<()> {
    logger::setup_logger();

    let config_path = config::discover_config_path()?;
    info!("Using configuration from: {}", config_path);

    let config = config::load_config::<LighthouseConfigFile>(config_path)?;
    let addr = config.lighthouse.get_listen_addr();

    info!("Lighthouse listening on: {}", addr);
    gotham::start(addr, router(config.lighthouse)?)?;

    Ok(())
}