mod app;
mod config;
mod controllers;
mod error;
mod extractors;
mod middleware;
mod models;
mod routes;
mod services;
mod state;

// entry point
#[tokio::main]
async fn main() {
    app::run().await;
}
