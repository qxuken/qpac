use std::{fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use debounced::debounced;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::{debug, error, info, span, trace, Level};

use crate::{
    error::{AppError, Result},
    pac::Pac,
    storage::{sqlite_storage::SqliteStorage, Storage},
    trace_layer,
};

mod auth;

#[derive(Debug)]
struct ServerState<S>
where
    S: Storage,
{
    storage: Arc<S>,
    update_tx: Sender<()>,
}

impl<S: Storage + Debug> ServerState<S> {
    fn new(storage: S, update_tx: Sender<()>) -> Self {
        Self {
            storage: Arc::new(storage),
            update_tx,
        }
    }
}

pub async fn run_web_server(
    bind: SocketAddr,
    token: Option<String>,
    database: Option<String>,
) -> Result<()> {
    tracing::debug!("Starting web server");

    let (update_tx, rx) = mpsc::channel(1);

    let storage = match database {
        Some(url) => SqliteStorage::new(&url).await?,
        None => SqliteStorage::new("sqlite::memory:").await?,
    };
    let server_state = Arc::new(ServerState::new(storage, update_tx));

    tokio::spawn(subscribe_pac(server_state.storage.clone(), rx));

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace_layer::trace_layer_make_span_with)
        .on_request(trace_layer::trace_layer_on_request)
        .on_response(trace_layer::trace_layer_on_response);
    let compression = CompressionLayer::new();

    let public = Router::new()
        .route("/list", get(get_list))
        .route("/", get(get_latest_pac))
        .route("/:hash", get(get_pac))
        .layer(compression);

    let mut admin = Router::new()
        .route("/add", post(add_to_list))
        .route("/remove", post(remove_from_list));
    if let Some(t) = token {
        admin = admin.route_layer(auth::use_auth_layer(t));
    } else {
        info!("Auth token is missing, running unsafe");
    }

    let app = Router::new()
        .merge(public)
        .merge(admin)
        .fallback(fallback)
        .layer(trace_layer)
        .with_state(server_state);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    tracing::info!("Listening on {}", bind);
    axum::serve(listener, app)
        .await
        .expect("Should start web server");

    Ok(())
}

#[tracing::instrument(skip_all, err(level = Level::DEBUG))]
async fn get_latest_pac(
    server_state: State<Arc<ServerState<impl Storage>>>,
) -> Result<Response<String>, AppError> {
    let pac = server_state.storage.get_file_latest().await?;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/javascript")
        .header(
            header::LOCATION,
            format!("/{}", urlencoding::encode(&pac.hash)),
        )
        .body(pac.file)
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tracing::instrument(skip_all, err(level = Level::DEBUG))]
async fn get_pac(
    Path(hash): Path<String>,
    server_state: State<Arc<ServerState<impl Storage>>>,
) -> Result<Response<String>, AppError> {
    let file = server_state.storage.get_file(hash).await?;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/javascript")
        .body(file)
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tracing::instrument(skip_all, err(level = Level::DEBUG))]
async fn get_list(
    server_state: State<Arc<ServerState<impl Storage>>>,
) -> Result<impl IntoResponse, AppError> {
    server_state.storage.all_hosts().await.map(Json)
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
    server_state.storage.add_host(props.host).await?;
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
    server_state.storage.remove_host(props.host).await?;
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

#[tracing::instrument(skip_all, err(Debug))]
async fn subscribe_pac(storage: Arc<impl Storage>, rx: Receiver<()>) -> Result<()> {
    let mut deb = debounced(ReceiverStream::new(rx), Duration::from_millis(150));
    while deb.next().await.is_some() {
        let s = span!(Level::TRACE, "update_tx");
        let _se = s.enter();
        debug!("recv");
        let hosts = match storage.all_hosts().await {
            Ok(v) => v,
            Err(e) => {
                error!("Error fetching hosts: {}", e);
                continue;
            }
        };
        trace!("generate");
        let pac = Pac::generate(hosts);

        trace!("upload");
        if let Err(e) = storage.upload_file(&pac).await {
            error!("Error saving file {}", e);
            continue;
        };

        trace!("set latest {}", &pac.hash);
        if let Err(e) = storage.set_latest(pac.hash).await {
            error!("Error setting latest {}", e);
            continue;
        };
    }
    Ok(())
}
