use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

pub fn get_subscriber(
    _name: String,
    env_filter: String,
    sink: impl Fn() -> Box<dyn std::io::Write + Send + Sync> + Send + Sync + 'static,
) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = fmt::layer().with_writer(sink).with_target(false).compact();
    Registry::default()
        .with(env_filter)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}