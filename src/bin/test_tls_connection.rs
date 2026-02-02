//! Test TLS certificate validation for cTrader endpoints.
//!
//! Usage: cargo run --bin test-tls-connection

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use x509_parser::prelude::{FromDer, X509Certificate};

#[derive(Debug, Clone, Copy)]
struct Target {
    host: &'static str,
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let targets = [
        Target {
            host: "live.ctraderapi.com",
            port: 5035,
        },
        Target {
            host: "demo.ctraderapi.com",
            port: 5035,
        },
    ];

    for target in targets {
        println!("\n=== TLS Validation: {}:{} ===", target.host, target.port);
        test_target(target).await?;
    }

    println!("\nAll TLS checks completed successfully.");
    Ok(())
}

async fn test_target(target: Target) -> Result<()> {
    let address = format!("{}:{}", target.host, target.port);
    let tcp_stream = timeout(Duration::from_secs(10), TcpStream::connect(&address))
        .await
        .with_context(|| format!("TCP connect timeout for {}", address))?
        .with_context(|| format!("TCP connect failed for {}", address))?;

    let tls_config = Arc::new(build_tls_config()?);
    let connector = TlsConnector::from(tls_config);

    let server_name = ServerName::try_from(target.host.to_string())
        .with_context(|| format!("Invalid DNS name: {}", target.host))?;

    let tls_stream = timeout(Duration::from_secs(10), connector.connect(server_name, tcp_stream))
        .await
        .with_context(|| format!("TLS handshake timeout for {}", address))?
        .with_context(|| format!("TLS handshake failed for {}", address))?;

    let (_, session) = tls_stream.get_ref();
    let protocol = session
        .protocol_version()
        .map(|v| format!("{:?}", v))
        .unwrap_or_else(|| "unknown".to_string());
    let cipher_suite = session
        .negotiated_cipher_suite()
        .map(|cs| format!("{:?}", cs))
        .unwrap_or_else(|| "unknown".to_string());

    println!("TLS handshake: OK");
    println!("Protocol: {}", protocol);
    println!("Cipher suite: {}", cipher_suite);

    let certs = session
        .peer_certificates()
        .map(|certs| certs.to_vec())
        .unwrap_or_default();

    if certs.is_empty() {
        bail!("No peer certificates presented by {}", target.host);
    }

    let leaf = &certs[0];
    print_certificate_details(leaf)?;

    Ok(())
}

fn build_tls_config() -> Result<ClientConfig> {
    let mut root_store = RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs()
        .context("Failed to load native root certificates")?;

    let mut added = 0usize;
    for cert in native_certs {
        if root_store.add(cert).is_ok() {
            added += 1;
        }
    }

    if added == 0 {
        bail!("No native root certificates were added");
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

fn print_certificate_details(cert_der: &rustls::pki_types::CertificateDer<'_>) -> Result<()> {
    let (_, cert) = X509Certificate::from_der(cert_der.as_ref())
        .context("Failed to parse leaf certificate")?;

    let subject = cert.subject();
    let issuer = cert.issuer();
    let validity = cert.validity();

    println!("Certificate subject: {}", subject);
    println!("Certificate issuer: {}", issuer);
    println!(
        "Validity: {} -> {}",
        validity.not_before.to_datetime(),
        validity.not_after.to_datetime()
    );

    if let Ok(Some(names)) = cert.subject_alternative_name() {
        let mut dns_names = Vec::new();
        for name in &names.value.general_names {
            if let x509_parser::extensions::GeneralName::DNSName(dns) = name {
                dns_names.push(dns.to_string());
            }
        }
        if !dns_names.is_empty() {
            println!("SANs: {}", dns_names.join(", "));
        }
    }

    Ok(())
}
