use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process;

use anyhow::{bail, Context};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::paths::Paths;
use crate::{ENDPOINT_GENERATION, PROTOCOL, VERSION};

const MANIFEST_CANDIDATES: &[&str] = &[
    "https://qatest.sh/latest.json",
    "https://github.com/Simonethg/qatest/releases/latest/download/latest.json",
    "https://raw.githubusercontent.com/Simonethg/qatest/main/latest.json",
];

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub protocol: u32,
    #[serde(default)]
    pub endpoint_generation: u32,
    #[serde(default)]
    pub notes: String,
    pub assets: std::collections::BTreeMap<String, String>,
    pub sha256: std::collections::BTreeMap<String, String>,
}

pub fn target_triple() -> anyhow::Result<String> {
    let os = std::env::consts::OS;
    let os = match os {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        other => bail!("unsupported OS: {other}"),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => bail!("unsupported architecture: {other}"),
    };
    Ok(format!("{os}-{arch}"))
}

pub fn fetch_manifest() -> anyhow::Result<Manifest> {
    if let Ok(url) = std::env::var("QATEST_MANIFEST_URL") {
        return fetch_url(&url);
    }
    let mut last = None;
    for url in MANIFEST_CANDIDATES {
        match fetch_url(url) {
            Ok(m) => return Ok(m),
            Err(e) => last = Some(format!("{url}: {e}")),
        }
    }
    bail!(
        "could not fetch latest.json ({})",
        last.unwrap_or_else(|| "no candidates".into())
    )
}

fn fetch_url(url: &str) -> anyhow::Result<Manifest> {
    let body = ureq::get(url)
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .with_context(|| format!("GET {url}"))?
        .into_string()?;
    Ok(serde_json::from_str(&body)?)
}

pub fn run_update() -> anyhow::Result<()> {
    let target = target_triple()?;
    let manifest = fetch_manifest()?;
    let url = manifest
        .assets
        .get(&target)
        .cloned()
        .with_context(|| format!("manifest has no asset for {target}"))?;
    let expected = manifest
        .sha256
        .get(&target)
        .cloned()
        .context("manifest missing sha256")?;
    let expected = expected.to_lowercase();
    if expected.len() != 64 || expected.chars().any(|c| !c.is_ascii_hexdigit()) {
        bail!("invalid sha256 in manifest");
    }

    eprintln!("qatest update  v{} → v{}  {target}", VERSION, manifest.version);
    let tmp = tempfile_path(&target)?;
    download(&url, &tmp)?;
    let actual = sha256_file(&tmp)?;
    if actual != expected {
        let _ = fs::remove_file(&tmp);
        bail!("checksum mismatch: got {actual} expected {expected}");
    }

    let paths = Paths::resolve()?;
    fs::create_dir_all(&paths.install_bin)?;
    let dest = paths.install_bin.join("qatest");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::copy(&tmp, &dest)?;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    {
        let dest = paths.install_bin.join("qatest.exe");
        fs::copy(&tmp, &dest)?;
    }
    let _ = fs::remove_file(&tmp);
    eprintln!(
        "installed {}  protocol={}  endpoint_generation={}",
        dest.display(),
        manifest.protocol.max(PROTOCOL),
        manifest.endpoint_generation.max(ENDPOINT_GENERATION)
    );
    eprintln!("ready. run 'qatest' to get started.");
    Ok(())
}

fn tempfile_path(target: &str) -> anyhow::Result<PathBuf> {
    let mut p = std::env::temp_dir().join(format!("qatest-update-{}-{}", process::id(), target));
    if cfg!(windows) {
        p.set_extension("bin");
    }
    Ok(p)
}

fn download(url: &str, dest: &PathBuf) -> anyhow::Result<()> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .with_context(|| format!("download {url}"))?;
    let mut reader = resp.into_reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    fs::write(dest, bytes)?;
    Ok(())
}

pub fn sha256_file(path: &PathBuf) -> anyhow::Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_os_arch() {
        let t = target_triple().unwrap();
        assert!(t.contains('-'));
    }
}
