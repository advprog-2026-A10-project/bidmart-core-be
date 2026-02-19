use axum::{Router, routing::get};

pub fn app() -> Router {
    Router::new().route("/", get(root))
}

pub async fn root() -> &'static str {
    "core_be is running with axum"
}

#[cfg(test)]
mod tests {
    use super::{app, root};

    #[test]
    fn app_builds_router() {
        let router = app();
        std::mem::drop(router);
    }

    #[tokio::test]
    async fn root_returns_expected_message() {
        assert_eq!(root().await, "core_be is running with axum");
    }
}
