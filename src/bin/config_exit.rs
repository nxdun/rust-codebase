// Test helper binary that runs AppConfig::from_env() in a separate process.
// Used to verify exit behavior when MASTER_API_KEY is missing.

use nadzu::config::AppConfig;

fn main() {
    let _ = AppConfig::from_env();
}
