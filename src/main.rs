use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind to 127.0.0.1:3000");

    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, core_be::app())
        .await
        .expect("server failed");
}
