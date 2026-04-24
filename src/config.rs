use std::fs;

use serde::Deserialize;

/// Runtime settings loaded from `config/server.yaml`.
#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    #[serde(default = "default_read_timeout_secs")]
    pub read_timeout_secs: u64,
    #[serde(default = "default_keep_alive_requests")]
    pub keep_alive_requests: usize,
}

/// Loads server runtime settings from a YAML file.
pub fn load_server_config(path: &str) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: ServerConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

fn default_worker_threads() -> usize {
    4
}

fn default_read_timeout_secs() -> u64 {
    5
}

fn default_keep_alive_requests() -> usize {
    8
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
        fs::write(
            &path,
            "host: 0.0.0.0\nport: 9000\nworker_threads: 6\nread_timeout_secs: 3\nkeep_alive_requests: 2\n",
        )
        .expect("should write config");

        let cfg = load_server_config(path.to_str().expect("path should be utf-8"))
            .expect("should load config");

        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 9000);
        assert_eq!(cfg.worker_threads, 6);
        assert_eq!(cfg.read_timeout_secs, 3);
        assert_eq!(cfg.keep_alive_requests, 2);

        fs::remove_file(path).expect("should remove temp file");
    }

    #[test]
    fn load_server_config_uses_defaults() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("server_config_defaults_{unique}.yaml"));
        fs::write(&path, "host: 127.0.0.1\nport: 7878\n").expect("should write config");

        let cfg = load_server_config(path.to_str().expect("path should be utf-8"))
            .expect("should load config");

        assert_eq!(cfg.worker_threads, 4);
        assert_eq!(cfg.read_timeout_secs, 5);
        assert_eq!(cfg.keep_alive_requests, 8);

        fs::remove_file(path).expect("should remove temp file");
    }
}
