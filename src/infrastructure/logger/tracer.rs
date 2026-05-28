use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracer() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,bidmart_core_be=debug,core_be.request=info,core_be.catalog.request=info,core_be.bidding.request=info,core_be.wallet.request=info,core_be.order.request=info,tower=debug,axum=info,sqlx=warn"
                    .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
