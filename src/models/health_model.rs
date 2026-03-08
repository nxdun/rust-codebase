use serde::Serialize;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
    pub cookies: bool,
}

impl Health {
    pub fn ok(cookies: bool) -> Self {
        Health {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            cookies,
        }
    }
}
