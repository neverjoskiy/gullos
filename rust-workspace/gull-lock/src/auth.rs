use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct LockConfig {
    pub user: String,
    pub hostname: String,
    password_sha256_hex: String,
}

impl LockConfig {
    pub fn load() -> Self {
        let mut user = env::var("USER").unwrap_or_else(|_| "gull".to_string());
        let mut hostname = fs::read_to_string("/etc/hostname")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "gull-pc".to_string());

        let mut default_plain = "gull".to_string();
        let mut explicit_hash: Option<String> = None;

        if let Ok(sys_cfg) = fs::read_to_string("/etc/gull-lock.conf") {
            for line in sys_cfg.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let key = k.trim();
                    let val = v.trim();
                    match key {
                        "user" if !val.is_empty() => user = val.to_string(),
                        "hostname" if !val.is_empty() => hostname = val.to_string(),
                        "default_password_mode" if !val.is_empty() => {
                            default_plain = val.to_string()
                        }
                        "password_sha256" if val.len() == 64 => {
                            explicit_hash = Some(val.to_ascii_lowercase())
                        }
                        _ => {}
                    }
                }
            }
        }

        let user_auth = user_auth_path();
        if let Ok(usr_cfg) = fs::read_to_string(&user_auth) {
            for line in usr_cfg.lines() {
                let trimmed = line.trim();
                if let Some((k, v)) = trimmed.split_once('=') {
                    if k.trim() == "password_sha256" && v.trim().len() == 64 {
                        explicit_hash = Some(v.trim().to_ascii_lowercase());
                    }
                }
            }
        }

        let password_sha256_hex = explicit_hash.unwrap_or_else(|| sha256_hex(default_plain.as_bytes()));

        Self {
            user,
            hostname,
            password_sha256_hex,
        }
    }

    pub fn verify(&self, candidate: &str) -> bool {
        let candidate_hash = sha256_hex(candidate.as_bytes());
        constant_time_eq(candidate_hash.as_bytes(), self.password_sha256_hex.as_bytes())
    }
}

pub fn interactive_set_password() -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(
        stdout,
        "\x1b[38;2;203;166;247m[gull-lock]\x1b[0m Enter new GullOS lockscreen password: "
    )?;
    stdout.flush()?;

    let mut pass1 = String::new();
    io::stdin().read_line(&mut pass1)?;
    let pass1 = pass1.trim_end_matches(&['\r', '\n'][..]);

    if pass1.is_empty() {
        eprintln!("gull-lock: password cannot be empty.");
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty password"));
    }

    let hash = sha256_hex(pass1.as_bytes());
    let path = user_auth_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &path,
        format!(
            "# GullOS user lockscreen credential hash\npassword_sha256={}\n",
            hash
        ),
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }

    println!(
        "\x1b[38;2;166;227;161m[gull-lock]\x1b[0m Updated lockscreen password hash at {}",
        path.display()
    );
    Ok(())
}

fn user_auth_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/gull".to_string());
    Path::new(&home).join(".config/gull-lock/auth.conf")
}

pub fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_default_gull_hash() {
        let cfg = LockConfig {
            user: "gull".into(),
            hostname: "gull-pc".into(),
            password_sha256_hex: sha256_hex(b"gull"),
        };
        assert!(cfg.verify("gull"));
        assert!(!cfg.verify("wrong"));
    }
}
