use axum::{
    routing::{get, delete},
    extract::{State, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use axum_macros::debug_handler;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use axum::response::Html;

const USERS_FILE: &str = "users.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    username: String,
    real_name: String,
    email: String,
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
}

#[derive(Error, Debug)]
enum ApiError {
    #[error("User not found")]
    UserNotFound,
    #[error("Internal server error")]
    ServerError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::UserNotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "User not found" })),
            ).into_response(),
            ApiError::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            ).into_response(),
        }
    }
}

// Load users from file
async fn load_users() -> Vec<User> {
    match tokio::fs::read_to_string(USERS_FILE).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// Save users to file
async fn save_users(users: &Vec<User>) -> Result<(), ApiError> {
    let data = serde_json::to_string_pretty(users).map_err(|_| ApiError::ServerError)?;
    tokio::fs::write(USERS_FILE, data).await.map_err(|_| ApiError::ServerError)?;
    Ok(())
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
    real_name: String,
    email: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    message: String,
}

#[debug_handler]
async fn list_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = state.users.lock().await;
    Json(users.clone())
}

#[debug_handler]
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<CreateUserResponse>, ApiError> {
    let mut users = state.users.lock().await;
    users.push(User {
        username: payload.username.clone(),
        real_name: payload.real_name.clone(),
        email: payload.email.clone(),
    });
    save_users(&users).await?;
    Ok(Json(CreateUserResponse {
        message: format!("User '{}' created!", payload.username),
    }))
}

#[debug_handler]
async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<usize>,
) -> Result<StatusCode, ApiError> {
    let mut users = state.users.lock().await;
    if user_id < users.len() {
        users.remove(user_id);
        save_users(&users).await?;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::UserNotFound)
    }
}

#[debug_handler]
async fn status(State(state): State<AppState>) -> Html<String> {
    let users = state.users.lock().await;
    let html = format!(
        r#"
        <h1>Server Status</h1>
        <p>Total users: {}</p>
        <ul>
            {}
        </ul>
        "#,
        users.len(),
        users.iter()
            .map(|u| format!("<li>{} ({}) - {}</li>", u.username, u.real_name, u.email))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Html(html)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let initial_users = load_users().await;
    let state = AppState {
        users: Arc::new(Mutex::new(initial_users)),
    };

    let app = Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/:user_id", delete(delete_user))
        .route("/status", get(status))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}