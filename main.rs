use axum::{
    routing::get,
    extract::{Query, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize)]
struct UpdateUser {
    username: Option<String>,
    email: Option<String>,
}

async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<usize>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<User>, ApiError> {
    let mut users = state.users.lock().map_err(|_| ApiError::ServerError)?;

    if let Some(user) = users.get_mut(user_id) {
        if let Some(new_username) = payload.username {
            user.username = new_username;
        }
        if let Some(new_email) = payload.email {
            user.email = new_email;
        }
        Ok(Json(user.clone()))
    } else {
        Err(ApiError::UserNotFound)
    }
}

async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<usize>,
) -> Result<StatusCode, ApiError> {
    let mut users = state.users.lock().map_err(|_| ApiError::ServerError)?;

    if user_id < users.len() {
        users.remove(user_id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::UserNotFound)
    }
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
            )
                .into_response(),

            ApiError::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Internal server error" })),
            )
                .into_response(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    username: String,
    email: String,
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<Vec<User>>>,
}

#[derive(Deserialize, Debug)]
struct CreateUser {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    message: String,
}

async fn list_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = state.users.lock().unwrap();
    Json(users.clone())
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Json<CreateUserResponse> {
    let mut users = state.users.lock().unwrap();
    users.push(User {
        username: payload.username.clone(),
        email: payload.email.clone(),
    });

    Json(CreateUserResponse {
        message: format!("User '{}' with email '{}' created!", payload.username, payload.email),
    })
}

async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<usize>,
) -> Result<impl IntoResponse, ApiError> {
    let users = state.users.lock().map_err(|_| ApiError::ServerError)?;

    if let Some(user) = users.get(user_id) {
        Ok(Json(json!({
            "username": user.username,
            "email": user.email
        })))
    } else {
        Err(ApiError::UserNotFound)
    }
}

#[derive(Deserialize)]
struct GreetParams {
    name: String,
}

async fn greet(Query(params): Query<GreetParams>) -> String {
    format!("Hello {}", params.name)
}

#[derive(Serialize)]
struct HelloResponse {
    message: String,
}

async fn root() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello from Spencer!".to_string(),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        users: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/greet", get(greet))
        .route("/users", get(list_users).post(create_user))
        .route("/users/:user_id", get(get_user).put(update_user).delete(delete_user))
        .with_state(state)
        .layer(TraceLayer::new_for_http());


    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}
