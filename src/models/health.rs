use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Health {
    pub status: &'static str,
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
