use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    #[serde(default = "default_pool_size")]
    pub max_connections: u32,
    pub protocol: String,
}

fn default_pool_size() -> u32 {
    50
}

impl DbConfig {
    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!(
                "{}://{}@{}:{}",
                self.protocol, self.user, self.host, self.port
            )
        } else {
            format!(
                "{}://{}:{}@{}:{}",
                self.protocol, self.user, self.password, self.host, self.port
            )
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }
}

impl DbConfig {
    pub fn load(filename: impl AsRef<Path>) -> Result<Self> {
        let config = fs::read_to_string(filename.as_ref())
            .map_err(|_| anyhow!("Failed to read db config"))?;
        serde_yaml::from_str(&config).map_err(|_| anyhow!("Failed to parse db config"))
    }
}
