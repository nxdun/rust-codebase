//! The root library module for Nadzu API.
//! This module exposes all core application modules.

/// Application startup and runtime orchestration.
pub mod app;
/// Configuration management.
pub mod config;
/// HTTP controllers for routing.
pub mod controllers;
/// Error types and handling.
pub mod error;
/// Request extractors.
pub mod extractors;
/// Tower middleware implementations.
pub mod middleware;
/// Data structures and domain models.
pub mod models;
/// Application routing definitions.
pub mod routes;
/// Business logic and core services.
pub mod services;
/// Global application state.
pub mod state;
