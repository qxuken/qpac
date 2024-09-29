use std::{fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, routing::get, Router};
use debounced::debounced;
use tokio::sync::{
    mpsc::{self, Receiver},
    Mutex,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::Level;

use crate::{
    error::Result,
    pac::Pac,
    storage::{memory_storage::MemoryStorage, Storage},
    trace_layer,
};

#[derive(Debug)]
struct ServerState<S>
where
    S: Storage,
{
    pac: Arc<Mutex<Pac>>,
    storage: Arc<Mutex<S>>,
}

impl<S: Storage + Debug> ServerState<S> {
    fn new(pac: Pac, storage: S) -> Self {
        Self {
            pac: Arc::new(Mutex::new(pac)),
            storage: Arc::new(Mutex::new(storage)),
        }
    }
}

pub async fn run_web_server(bind: SocketAddr) -> Result<()> {
    tracing::debug!("Starting web server");

    let (_update_tx, rx) = mpsc::channel(1);

    let storage = MemoryStorage::default();
    let pac = Pac::new(storage.all_hosts()?);
    let server_state = Arc::new(ServerState::new(pac, storage));

    tokio::spawn(subscribe_pac(
        server_state.storage.clone(),
        server_state.pac.clone(),
        rx,
    ));

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace_layer::trace_layer_make_span_with)
        .on_request(trace_layer::trace_layer_on_request)
        .on_response(trace_layer::trace_layer_on_response);
    let compression = CompressionLayer::new();

    let app = Router::new()
        .route("/", get(get_list))
        .fallback(fallback)
        .with_state(server_state)
        .layer(compression)
        .layer(trace_layer);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    tracing::info!("Listening on {}", bind);
    axum::serve(listener, app)
        .await
        .expect("Should start web server");

    Ok(())
}

#[tracing::instrument(skip_all, ret(level = Level::TRACE))]
async fn get_list(server_state: State<Arc<ServerState<impl Storage>>>) -> String {
    server_state.pac.lock().await.get_file()
}

#[tracing::instrument]
async fn fallback() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not Found")
}

async fn subscribe_pac(
    storage: Arc<Mutex<impl Storage>>,
    pac: Arc<Mutex<Pac>>,
    rx: Receiver<()>,
) -> Result<()> {
    let mut deb = debounced(ReceiverStream::new(rx), Duration::from_millis(150));
    while deb.next().await.is_some() {
        tracing::trace!("update req recv");
        let hosts = storage.lock().await.all_hosts()?;
        pac.lock().await.update(hosts);
    }
    Ok(())
}
