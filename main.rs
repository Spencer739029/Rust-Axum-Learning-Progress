use axum::{routing::get, Json, Router};
use axum::routing::post;
use serde::{Serialize, Deserialize};
use axum::extract::{Query, Path, State};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

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
) -> String {
    let users = state.users.lock().unwrap();

    if let Some(user) = users.get(user_id) {
        format!("Found user: {} ({})", user.username, user.email)
    } else {
        format!("No user found with index {}", user_id)
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

#[tokio::main]
async fn main() {
    let state = AppState {
        users: Arc::new(Mutex::new(Vec::new())),
    };

    let app = Router::new()
    .route("/", get(root))
    .route("/greet", get(greet))
    .route("/users/:user_id", get(get_user))
    .route("/users", post(create_user))
    .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn root() -> Json<HelloResponse> {
    Json(HelloResponse {
        message: "Hello from Spencer!".to_string(),
    })
}
