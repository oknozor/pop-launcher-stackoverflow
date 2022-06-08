use serde::Deserialize;

use anyhow::{anyhow, Result};
use pop_launcher_toolkit::launcher::config::find;

#[derive(Deserialize)]
struct PluginConfig {
    access_token: String,
}

pub fn access_token() -> Result<String> {
    let config = find("stackoverflow")
        .find(|path| path.exists())
        .ok_or_else(|| anyhow!("'config.ron' config file not found for stackoverflow plugin"));

    let config = config?;
    let config = std::fs::read_to_string(config)?;
    let config: PluginConfig = ron::from_str(&config)?;
    Ok(config.access_token)
}
