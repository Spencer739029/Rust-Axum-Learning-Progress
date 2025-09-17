use axum::{routing::get, Json, Router};
use axum::routing::post;
use serde::{Serialize, Deserialize};
use axum::extract::{Query, Path};
use std::net::SocketAddr;

#[derive(Deserialize, Debug)]
struct CreateUser {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    message: String,
}

async fn create_user(Json(payload): Json<CreateUser>) -> Json<CreateUserResponse> {
    Json(CreateUserResponse {
        message: format!("User '{}' with email '{}' created!", payload.username, payload.email),
    })
}

async fn get_user(Path(user_id): Path<u32>) -> String {
    format!("User ID: {}", user_id)
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
    let app = Router::new()
    .route("/", get(root))
    .route("/greet", get(greet))
    .route("/users/:user_id", get(get_user))
    .route("/users", post(create_user));

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
