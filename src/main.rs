use axum::{Router, routing::get};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(root));

    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind to 127.0.0.1:3000");

    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server failed");
}

async fn root() -> &'static str {
    "core_be is running with axum"
}
