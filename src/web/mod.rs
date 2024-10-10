use std::{fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use debounced::debounced;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::Level;

use crate::{
    error::{AppError, Result},
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
    update_tx: Sender<()>,
}

impl<S: Storage + Debug> ServerState<S> {
    fn new(pac: Pac, storage: S, update_tx: Sender<()>) -> Self {
        Self {
            pac: Arc::new(Mutex::new(pac)),
            storage: Arc::new(Mutex::new(storage)),
            update_tx,
        }
    }
}

pub async fn run_web_server(bind: SocketAddr) -> Result<()> {
    tracing::debug!("Starting web server");

    let (update_tx, rx) = mpsc::channel(1);

    let storage = MemoryStorage::default();
    let pac = Pac::new(storage.all_hosts()?);
    let server_state = Arc::new(ServerState::new(pac, storage, update_tx));

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
        .route("/", get(get_pac))
        .route("/list", get(get_list))
        .route("/add", post(add_to_list))
        .route("/remove", post(remove_from_list))
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
async fn get_pac(server_state: State<Arc<ServerState<impl Storage>>>) -> impl IntoResponse {
    server_state.pac.lock().await.get_file()
}

#[tracing::instrument(skip_all, ret(level = Level::TRACE))]
async fn get_list(
    server_state: State<Arc<ServerState<impl Storage>>>,
) -> Result<impl IntoResponse, AppError> {
    server_state.storage.lock().await.all_hosts().map(Json)
}

#[derive(Debug, Deserialize)]
struct HostProps {
    host: String,
}

#[tracing::instrument(skip(server_state), ret(level = Level::TRACE))]
async fn add_to_list(
    server_state: State<Arc<ServerState<impl Storage>>>,
    Json(props): Json<HostProps>,
) -> Result<impl IntoResponse, AppError> {
    server_state.storage.lock().await.add_host(props.host)?;
    server_state
        .update_tx
        .send(())
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(Json(json!({ "success": true })))
}

#[tracing::instrument(skip(server_state), ret(level = Level::TRACE))]
async fn remove_from_list(
    server_state: State<Arc<ServerState<impl Storage>>>,
    Json(props): Json<HostProps>,
) -> Result<impl IntoResponse, AppError> {
    server_state.storage.lock().await.remove_host(props.host)?;
    server_state
        .update_tx
        .send(())
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;
    Ok(Json(json!({ "success": true })))
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
