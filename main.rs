use axum::{
    routing::{get, post, delete},
    extract::{State, Path, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response, Html},
    Json, Router,
    serve,
};
use axum_macros::debug_handler;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc, collections::HashMap};
use tokio::sync::Mutex;
use tokio::net::TcpListener;
use uuid::Uuid;
use tokio::fs;
use async_trait::async_trait;

const USERS_FILE: &str = "users.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    username: String,
    real_name: String,
    email: String,
    created_by: String, // who made this user
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
    sessions: Arc<Mutex<HashMap<String, String>>>, // token -> username
}

#[derive(Error, Debug)]
enum ApiError {
    #[error("User not found")]
    UserNotFound,
    #[error("Forbidden")]
    Forbidden,
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
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "You are not allowed to do this action" })),
            ).into_response(),
            ApiError::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            ).into_response(),
        }
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

#[debug_handler]
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let token = Uuid::new_v4().to_string();
    let mut sessions = state.sessions.lock().await;
    sessions.insert(token.clone(), payload.username.clone());

    Ok(Json(LoginResponse { token }))
}

struct SessionToken {
    username: String,
}

#[async_trait]
impl FromRequestParts<AppState> for SessionToken {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if let Some(token) = parts.headers.get("x-session-token") {
            if let Ok(token_str) = token.to_str() {
                let sessions = state.sessions.lock().await;
                if let Some(username) = sessions.get(token_str) {
                    return Ok(SessionToken {
                        username: username.clone(),
                    });
                }
            }
        }
        Err(ApiError::Forbidden)
    }
}

// Load users from file
async fn load_users() -> Vec<User> {
    match fs::read_to_string(USERS_FILE).await {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// Save users to file
async fn save_users(users: &Vec<User>) -> Result<(), ApiError> {
    let data = serde_json::to_string_pretty(users).map_err(|_| ApiError::ServerError)?;
    fs::write(USERS_FILE, data).await.map_err(|_| ApiError::ServerError)?;
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
    token: SessionToken,
    Json(payload): Json<CreateUser>,
) -> Result<Json<CreateUserResponse>, ApiError> {
    println!("Creating user: {} by {}", payload.username, token.username);
    let mut users = state.users.lock().await;
    users.push(User {
        username: payload.username.clone(),
        real_name: payload.real_name.clone(),
        email: payload.email.clone(),
        created_by: token.username.clone(),
    });
    save_users(&users).await?;
    println!("User created successfully");
    Ok(Json(CreateUserResponse {
        message: format!("User '{}' created!", payload.username),
    }))
}

#[debug_handler]
async fn delete_user(
    State(state): State<AppState>,
    token: SessionToken,
    Path(user_id): Path<usize>,
) -> Result<StatusCode, ApiError> {
    let mut users = state.users.lock().await;
    if user_id < users.len() {
        if users[user_id].created_by == token.username {
            users.remove(user_id);
            save_users(&users).await?;
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(ApiError::Forbidden)
        }
    } else {
        Err(ApiError::UserNotFound)
    }
}

#[debug_handler]
async fn status() -> Html<String> {
    match fs::read_to_string("status.html").await {
        Ok(contents) => Html(contents),
        Err(_) => Html("<h1>Error loading status page</h1>".to_string()),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let initial_users = load_users().await;
    let state = AppState {
        users: Arc::new(Mutex::new(initial_users)),
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/login", post(login))
        .route("/users", get(list_users).post(create_user))
        .route("/users/:user_id", delete(delete_user))
        .route("/status", get(status))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();
    serve(listener, app).await.unwrap();
}