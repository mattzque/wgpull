use env_logger::Builder;
use log::LevelFilter;

pub fn setup_logger() {
    let mut builder = Builder::new();

    // set the default level to info when none is specified
    builder.filter(None, LevelFilter::Info);

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    }

    builder.init();
}
