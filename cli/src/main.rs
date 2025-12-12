// SPDX-FileCopyrightText: Alice Frosi <afrosi@redhat.com>
//
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use clap::{Parser, Subcommand};
use clevis_pin_trustee_lib::*;
use josekit::jwe::alg::direct::DirectJweAlgorithm::Dir;
use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command as StdCommand;
use std::time::Duration;
use std::{fs, thread};

const DEFAULT_TRIES: u32 = 10;
const DELAY: Duration = Duration::from_secs(5);

/// Trait for executing commands to fetch LUKS keys
trait CommandExecutor {
    fn try_fetch_luks_key(
        &self,
        url: &str,
        path: &str,
        cert: &str,
        initdata: Option<String>,
    ) -> Result<String>;
}

/// Real implementation that calls the trustee-attester binary
struct RealCommandExecutor;

impl CommandExecutor for RealCommandExecutor {
    fn try_fetch_luks_key(
        &self,
        url: &str,
        path: &str,
        cert: &str,
        initdata: Option<String>,
    ) -> Result<String> {
        let mut command = StdCommand::new("trustee-attester");
        if !cert.is_empty() {
            // Create a unique filename based on the URL
            let url_sanitized = url.replace("://", "_").replace("/", "_").replace(":", "_");
            let cert_path = format!("/var/run/trustee/cert_{}.pem", url_sanitized);
            let cert_path_obj = Path::new(&cert_path);
            if let Some(parent) = cert_path_obj.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&cert_path, cert)?;
            command.arg("--cert-file").arg(&cert_path);
        }
        command
            .arg("--url")
            .arg(url)
            .arg("get-resource")
            .arg("--path")
            .arg(path);
        if let Some(initdata_str) = initdata {
            command.arg("--initdata").arg(initdata_str);
        }
        let output = command
            .output()
            .map_err(|e| anyhow!("Failed to execute trustee-attester: {}", e))?;

        io::stderr().write_all(&output.stderr)?;
        io::stderr().write_all(&output.stdout)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("trustee-attester failed: {}", stderr));
        }

        let key = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Invalid UTF-8 for the LUKS key: {}", e))?
            .trim()
            .to_string();

        if key.is_empty() {
            return Err(anyhow!("Received empty LUKS key"));
        }

        Ok(key)
    }
}

#[cfg(test)]
pub struct MockCommandExecutor {
    pub response: Result<String>,
}

#[cfg(test)]
impl CommandExecutor for MockCommandExecutor {
    fn try_fetch_luks_key(
        &self,
        _url: &str,
        _path: &str,
        _cert: &str,
        _initdata: Option<String>,
    ) -> Result<String> {
        match &self.response {
            Ok(key) => Ok(key.clone()),
            Err(e) => Err(anyhow!("{}", e)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ClevisHeader {
    pin: String,
    servers: Vec<Server>,
    path: String,
    initdata: Option<String>,
    #[serde(default)]
    num_retries: Option<NumRetries>,
}

fn fetch_and_prepare_jwk<E: CommandExecutor>(
    servers: &[Server],
    path: &str,
    initdata: Option<String>,
    num_retries: &NumRetries,
    executor: &E,
) -> Result<Jwk> {
    let key = fetch_luks_key(servers, path, initdata, num_retries, executor)?;
    let key = String::from_utf8(
        general_purpose::STANDARD
            .decode(&key)
            .context("Error decoding key in base64")?,
    )
    .context("Error decoding the key in JSON")?;
    eprintln!("Key: {:?}", key);
    let key: Key = serde_json::from_str(&key).context("Error in parsing the fetched key")?;

    let mut jwk = Jwk::new(&key.key_type);
    jwk.set_key_value(&key.key);
    jwk.set_key_operations(vec!["encrypt", "decrypt"]);

    Ok(jwk)
}

fn encrypt(config: &str) -> Result<()> {
    let config: Config =
        serde_json::from_str(config).map_err(|e| anyhow!("Failed to parse config JSON: {}", e))?;
    let initdata_str = config.initdata.as_ref();
    let initdata_data: Option<HashMap<String, String>> = initdata_str
        .map(|s| {
            serde_json::from_str(s).map_err(|e| anyhow!("Failed to parse config initdata: {e}"))
        })
        .transpose()?;
    let initdata = initdata_data
        .map(|data| {
            toml::to_string(&Initdata {
                version: "0.1.0".to_string(),
                algorithm: "sha256".to_string(),
                data,
            })
            .map_err(|e| anyhow!("Failed to serialize initdata: {e}"))
        })
        .transpose()?;

    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let executor = RealCommandExecutor;
    let num_retries = config
        .num_retries
        .as_ref()
        .unwrap_or(&NumRetries::Finite(DEFAULT_TRIES));
    let jwk = fetch_and_prepare_jwk(
        &config.servers,
        &config.path,
        initdata.clone(),
        num_retries,
        &executor,
    )?;

    eprintln!("{}", jwk);
    let encrypter = Dir
        .encrypter_from_jwk(&jwk)
        .context("Error creating direct encrypter")?;

    let private_hdr = ClevisHeader {
        pin: "trustee".to_string(),
        servers: config.servers.clone(),
        path: config.path,
        initdata,
        num_retries: config.num_retries,
    };

    let mut hdr = josekit::jwe::JweHeader::new();
    hdr.set_algorithm("ECDH-ES");
    hdr.set_content_encryption("A256GCM");
    hdr.set_claim(
        "clevis",
        Some(serde_json::value::to_value(private_hdr).context("Error serializing private header")?),
    )
    .context("Error adding clevis claim")?;

    let jwe_token = josekit::jwe::serialize_compact(&input, &hdr, &encrypter)
        .context("Error serializing JWE token")?;

    io::stdout()
        .write_all(jwe_token.as_bytes())
        .context("Error writing the token on stdout")?;
    eprintln!("Encryption successful.");

    Ok(())
}

fn decrypt() -> Result<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;
    let input = std::str::from_utf8(&input).context("Input is not valid UTF-8")?;

    let hdr = josekit::jwt::decode_header(input).context("Error decoding header")?;
    let hdr_clevis = hdr.claim("clevis").context("Error getting clevis claim")?;
    let hdr_clevis: ClevisHeader =
        serde_json::from_value(hdr_clevis.clone()).context("Error deserializing clevis header")?;

    eprintln!("Decrypt with header: {:?}", hdr_clevis);

    let executor = RealCommandExecutor;
    let num_retries = hdr_clevis
        .num_retries
        .as_ref()
        .unwrap_or(&NumRetries::Finite(DEFAULT_TRIES));
    let decrypter_jwk = fetch_and_prepare_jwk(
        &hdr_clevis.servers,
        &hdr_clevis.path,
        hdr_clevis.initdata,
        num_retries,
        &executor,
    )?;

    let decrypter = Dir
        .decrypter_from_jwk(&decrypter_jwk)
        .context("Error creating decrypter")?;

    let (payload, _) =
        josekit::jwe::deserialize_compact(input, &decrypter).context("Error decrypting JWE")?;

    io::stdout().write_all(&payload)?;

    eprintln!("Decryption successful.");
    Ok(())
}

fn try_fetch_from_servers<E: CommandExecutor>(
    servers: &[Server],
    path: &str,
    initdata: &Option<String>,
    executor: &E,
) -> Option<String> {
    for (index, server) in servers.iter().enumerate() {
        eprintln!("Trying URL {}/{}: {}", index + 1, servers.len(), server.url);
        match executor.try_fetch_luks_key(&server.url, path, &server.cert, initdata.clone()) {
            Ok(key) => {
                eprintln!("Successfully fetched LUKS key from URL: {}", server.url);
                return Some(key);
            }
            Err(e) => {
                eprintln!("Error with URL {}: {}", server.url, e);
            }
        }
    }
    None
}

fn fetch_luks_key<E: CommandExecutor>(
    servers: &[Server],
    path: &str,
    initdata: Option<String>,
    num_retries: &NumRetries,
    executor: &E,
) -> Result<String> {
    if servers.is_empty() {
        return Err(anyhow!("No URLs provided"));
    }

    match num_retries {
        NumRetries::Finite(max_attempts) => (1..=*max_attempts)
            .find_map(|attempt| {
                eprintln!(
                    "Attempting to fetch LUKS key (attempt {}/{})",
                    attempt, max_attempts
                );

                if let Some(key) = try_fetch_from_servers(servers, path, &initdata, executor) {
                    return Some(Ok(key));
                }

                if attempt < *max_attempts {
                    eprintln!(
                        "All URLs failed for attempt {}. Retrying in {:?} seconds...",
                        attempt, DELAY
                    );
                    thread::sleep(DELAY);
                }
                None
            })
            .unwrap_or_else(|| {
                Err(anyhow!(
                    "Failed to fetch the LUKS key from all URLs after {} attempts",
                    max_attempts
                ))
            }),
        NumRetries::Infinity => {
            let mut attempt = 0;
            loop {
                attempt += 1;
                eprintln!("Attempting to fetch LUKS key (attempt {})", attempt);

                if let Some(key) = try_fetch_from_servers(servers, path, &initdata, executor) {
                    return Ok(key);
                }

                eprintln!(
                    "All URLs failed for attempt {}. Retrying in {:?} seconds...",
                    attempt, DELAY
                );
                thread::sleep(DELAY);
            }
        }
    }
}

/// Clevis PIN for Trustee
#[derive(Parser)]
#[command(name = "clevis-pin-trustee")]
#[command(version = "0.1.0")]
#[command(about = "Clevis PIN for Trustee")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encrypt data using the configuration
    Encrypt {
        /// Input data or arguments
        config: String,
    },
    /// Decrypt the input data
    Decrypt,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encrypt { config } => encrypt(&config),
        Commands::Decrypt => decrypt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_luks_key_success() {
        let mock = MockCommandExecutor {
            response: Ok("test_luks_key_12345".to_string()),
        };

        let servers = vec![Server {
            url: "http://server1.example.com".to_string(),
            cert: String::new(),
        }];

        let num_retries = NumRetries::Finite(3);
        let result = fetch_luks_key(&servers, "/test/path", None, &num_retries, &mock);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_luks_key_12345");
    }

    #[test]
    fn test_fetch_luks_key_error() {
        let mock = MockCommandExecutor {
            response: Err(anyhow!("Failed to connect to server")),
        };

        let servers = vec![Server {
            url: "http://server1.example.com".to_string(),
            cert: String::new(),
        }];

        let num_retries = NumRetries::Finite(3);
        let result = fetch_luks_key(&servers, "/test/path", None, &num_retries, &mock);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to fetch the LUKS key from all URLs after 3 attempts"
        );
    }

    #[test]
    fn test_fetch_luks_key_infinity_retries() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use std::time::Instant;

        let mock = MockCommandExecutor {
            response: Err(anyhow!("Failed to connect to server")),
        };

        let servers = vec![Server {
            url: "http://server1.example.com".to_string(),
            cert: String::new(),
        }];

        let num_retries = NumRetries::Infinity;

        let returned = Arc::new(AtomicBool::new(false));
        let returned_clone = Arc::clone(&returned);
        let handle = std::thread::spawn(move || {
            let _ = fetch_luks_key(&servers, "/test/path", None, &num_retries, &mock);
            returned_clone.store(true, Ordering::SeqCst);
        });
        let start = Instant::now();
        let timeout = Duration::from_secs(60);

        while start.elapsed() < timeout {
            thread::sleep(Duration::from_secs(1));
            if returned.load(Ordering::SeqCst) {
                panic!("fetch_luks_key returned before 1 minute with infinite retries");
            }
        }

        assert!(
            !returned.load(Ordering::SeqCst),
            "fetch_luks_key should not have returned after 1 minute with infinite retries"
        );

        drop(handle);
    }
}
