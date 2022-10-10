use tracing_subscriber::filter::{EnvFilter, LevelFilter};

/// Bootstrap function to be defined before each test case.
/// Automaticlly executed, don't call this manually.
#[ctor::ctor]
fn before_each() {
    // Init loggers for test
    let log_subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::TRACE.into())
                .from_env_lossy()
                .add_directive("hyper=info".parse().unwrap())
                .add_directive("tokio=info".parse().unwrap()),
        )
        .with_line_number(true)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(log_subscriber)
        .expect("Setting default subscriber failed");
}
