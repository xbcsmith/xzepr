// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/infrastructure/tls/config.rs
use rustls::{ServerConfig, PrivateKey, Certificate};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;

pub struct TlsConfig {
    cert_path: String,
    key_path: String,
}

impl TlsConfig {
    pub fn load_server_config(&self) -> Result<ServerConfig, TlsError> {
        // Load certificates
        let cert_file = File::open(&self.cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<Certificate> = certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = File::open(&self.key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)?;

        if keys.is_empty() {
            return Err(TlsError::NoPrivateKey);
        }

        let key = PrivateKey(keys.remove(0));

        // Configure TLS 1.3 only
        let config = ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("TLS 1.3 should be supported")
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}

// src/main.rs - Server startup with TLS
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS config
    let tls_config = TlsConfig::new(
        "certs/server.crt".to_string(),
        "certs/server.key".to_string(),
    );

    let rustls_config = RustlsConfig::from_config(
        Arc::new(tls_config.load_server_config()?)
    );

    // Build app
    let app = build_app().await?;

    // Start HTTPS server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));
    tracing::info!("Starting HTTPS server on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
