use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

pub fn load_server_config(path: &str) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: ServerConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::load_server_config;

    #[test]
    fn load_server_config_from_yaml() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("server_config_{unique}.yaml"));
        fs::write(&path, "host: 0.0.0.0\nport: 9000\n").expect("should write config");

        let cfg = load_server_config(path.to_str().expect("path should be utf-8"))
            .expect("should load config");

        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 9000);

        fs::remove_file(path).expect("should remove temp file");
    }
}
