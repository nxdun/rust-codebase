use serde::Serialize;

/// Represents the health status of the application.
#[derive(Serialize, Debug)]
pub struct Health {
    /// The current status ("ok").
    pub status: &'static str,
    /// The version of the application.
    pub version: &'static str,
}

impl Health {
    /// Creates a new Health instance with "ok" status and current version.
    #[must_use]
    pub const fn ok() -> Self {
        Self {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}
