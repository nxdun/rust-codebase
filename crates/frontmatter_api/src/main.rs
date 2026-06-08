pub mod app;
pub mod controllers;
pub mod extractors;
pub mod middleware;
pub mod routes;
pub mod services;
pub mod state;
pub mod telemetry;

#[tokio::main]
async fn main() {
    app::run().await;
}