use std::sync::Arc;

use rust_ingress::config::Config;
use rust_ingress::kafka::{MockProducer, Producer};
use rust_ingress::server::{router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = Config::from_env().unwrap_or_else(|e| {
        eprintln!("config error: {}", e);
        std::process::exit(2);
    });

    let producer: Arc<dyn Producer> = if cfg.use_kafka {
        #[cfg(feature = "kafka")]
        {
            Arc::new(
                rust_ingress::kafka::RdkafkaProducer::new(&cfg.kafka_brokers)
                    .unwrap_or_else(|e| {
                        eprintln!("kafka error: {}", e);
                        std::process::exit(3);
                    }),
            )
        }
        #[cfg(not(feature = "kafka"))]
        {
            eprintln!("built without the `kafka` feature; set RUST_INGRESS_PRODUCER=mock");
            std::process::exit(4);
        }
    } else {
        Arc::new(MockProducer::new())
    };

    let state = Arc::new(AppState {
        cfg,
        producer,
        seen_meta: std::sync::Mutex::new(std::collections::HashSet::new()),
    });

    let listener = tokio::net::TcpListener::bind(&state.cfg.listen)
        .await
        .unwrap_or_else(|e| {
            eprintln!("bind error: {}", e);
            std::process::exit(1);
        });
    tracing::info!("listening on {}", state.cfg.listen);
    axum::serve(listener, router(state)).await.unwrap();
}
