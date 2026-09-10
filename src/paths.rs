use std::path::{Path, PathBuf};

/// XDG-style paths. State lives in ~/.local/share/qat — no Postgres.
#[derive(Debug, Clone)]
pub struct Paths {
    pub data: PathBuf,
    pub socket: PathBuf,
    pub pid: PathBuf,
    pub log: PathBuf,
    pub install_bin: PathBuf,
}

impl Paths {
    pub fn resolve() -> anyhow::Result<Self> {
        let data = if let Some(custom) = std::env::var_os("QAT_DATA_DIR") {
            PathBuf::from(custom)
        } else {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from(".").join(".local").join("share"))
                .join("qat")
        };
        let install_bin = if let Some(custom) = std::env::var_os("QAT_INSTALL_DIR") {
            PathBuf::from(custom)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("bin")
        };
        Ok(Self {
            socket: data.join("qat.sock"),
            pid: data.join("qat.pid"),
            log: data.join("qat.log"),
            data,
            install_bin,
        })
    }

    pub fn ensure(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.data)?;
        Ok(())
    }

    pub fn workspace_dot_qat(cwd: &Path) -> PathBuf {
        cwd.join(".qat")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_lives_under_data() {
        let p = Paths::resolve().unwrap();
        assert!(p.socket.starts_with(&p.data));
    }
}
