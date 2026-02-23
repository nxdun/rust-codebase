use nadzu::app;

// entry point
#[tokio::main]
async fn main() {
    app::run().await;
}
