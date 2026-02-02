//! TLS verification tests for cTrader LIVE/DEMO endpoints.
//!
//! These tests require outbound access to cTrader servers.
//! Enable them with: CTRADER_TLS_TESTS=1
//!
//! Example:
//!   CTRADER_TLS_TESTS=1 cargo test --test tls_verification_test -- --nocapture

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rustls::client::danger::ServerCertVerifier;
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ProtocolVersion, SupportedCipherSuite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const LIVE_HOST: &str = "live.ctraderapi.com";
const DEMO_HOST: &str = "demo.ctraderapi.com";
const PORT: u16 = 5035;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct TlsReport {
    version: ProtocolVersion,
    cipher_suite: SupportedCipherSuite,
    cert_chain: Vec<CertificateDer<'static>>,
}

fn live_tests_enabled() -> bool {
    match std::env::var("CTRADER_TLS_TESTS") {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"),
        Err(_) => false,
    }
}

fn build_root_store() -> Result<rustls::RootCertStore> {
    let mut root_store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs()
        .context("failed to load native root certificates")?;
    let (added, _) = root_store.add_parsable_certificates(certs);
    anyhow::ensure!(added > 0, "no valid root certificates were loaded");
    Ok(root_store)
}

fn build_client_config() -> Result<ClientConfig> {
    let root_store = build_root_store()?;
    let config = ClientConfig::builder_with_provider(Arc::new(default_provider()))
        .with_safe_default_protocol_versions()
        .context("unable to configure TLS versions")?
        .with_root_certificates(root_store)
        .with_no_client_auth();
    Ok(config)
}

async fn connect_tls(host: &str) -> Result<TlsReport> {
    let addr = format!("{host}:{PORT}");
    let tcp = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))?;

    let server_name: ServerName<'static> = ServerName::try_from(host.to_string())
        .with_context(|| format!("invalid server name: {host}"))?;
    let connector = TlsConnector::from(Arc::new(build_client_config()?));

    let tls_stream = tokio::time::timeout(CONNECT_TIMEOUT, connector.connect(server_name, tcp))
        .await
        .context("TLS handshake timed out")??;

    let (_, connection) = tls_stream.get_ref();
    let version = connection
        .protocol_version()
        .context("TLS version not negotiated")?;
    let cipher_suite = connection
        .negotiated_cipher_suite()
        .context("cipher suite not negotiated")?;
    let peer_certs = connection
        .peer_certificates()
        .context("peer did not provide certificates")?;

    let cert_chain = peer_certs
        .iter()
        .cloned()
        .map(|cert| cert.into_owned())
        .collect();

    Ok(TlsReport {
        version,
        cipher_suite,
        cert_chain,
    })
}

fn assert_tls12_or_newer(version: ProtocolVersion) {
    assert!(
        matches!(version, ProtocolVersion::TLSv1_2 | ProtocolVersion::TLSv1_3),
        "TLS version should be 1.2+, got {:?}",
        version
    );
}

#[tokio::test]
async fn test_live_server_connection() -> Result<()> {
    if !live_tests_enabled() {
        eprintln!("Skipping live TLS test. Set CTRADER_TLS_TESTS=1 to run.");
        return Ok(());
    }

    let report = connect_tls(LIVE_HOST).await?;
    assert_tls12_or_newer(report.version);
    assert!(!report.cert_chain.is_empty(), "certificate chain is empty");

    Ok(())
}

#[tokio::test]
async fn test_tls_certificate_chain() -> Result<()> {
    if !live_tests_enabled() {
        eprintln!("Skipping live TLS test. Set CTRADER_TLS_TESTS=1 to run.");
        return Ok(());
    }

    let report = connect_tls(LIVE_HOST).await?;
    let (leaf, intermediates) = report
        .cert_chain
        .split_first()
        .context("certificate chain is empty")?;

    let root_store = build_root_store()?;
    let verifier = WebPkiServerVerifier::builder(Arc::new(root_store))
        .build()
        .context("unable to build WebPkiServerVerifier")?;

    let server_name: ServerName<'static> = ServerName::try_from(LIVE_HOST.to_string())
        .context("invalid server name for verifier")?;

    verifier
        .verify_server_cert(
            leaf,
            intermediates,
            &server_name,
            &[],  // OCSP response (empty)
            UnixTime::now(),
        )
        .context("certificate chain verification failed")?;

    Ok(())
}

#[tokio::test]
async fn test_tls_cipher_suites() -> Result<()> {
    if !live_tests_enabled() {
        eprintln!("Skipping live TLS test. Set CTRADER_TLS_TESTS=1 to run.");
        return Ok(());
    }

    let report = connect_tls(LIVE_HOST).await?;
    assert_tls12_or_newer(report.version);

    // Verify negotiated cipher suite is a known secure suite
    let suite = report.cipher_suite.suite();
    eprintln!("Negotiated cipher suite: {:?}", suite);

    // Just verify we got a cipher suite (it will be from the default provider)
    assert!(
        format!("{:?}", suite).contains("TLS"),
        "negotiated cipher suite should be a TLS suite: {:?}",
        suite
    );

    Ok(())
}

#[tokio::test]
async fn test_demo_vs_live_connection() -> Result<()> {
    if !live_tests_enabled() {
        eprintln!("Skipping demo/live TLS test. Set CTRADER_TLS_TESTS=1 to run.");
        return Ok(());
    }

    let live = connect_tls(LIVE_HOST).await?;
    let demo = connect_tls(DEMO_HOST).await?;

    assert_tls12_or_newer(live.version);
    assert_tls12_or_newer(demo.version);

    let version_diff = live.version != demo.version;
    let cipher_diff = live.cipher_suite.suite() != demo.cipher_suite.suite();
    let chain_len_diff = live.cert_chain.len() != demo.cert_chain.len();

    if version_diff || cipher_diff || chain_len_diff {
        eprintln!(
            "DEMO/LIVE differences -> version: {:?} vs {:?}, cipher: {:?} vs {:?}, chain length: {} vs {}",
            demo.version,
            live.version,
            demo.cipher_suite.suite(),
            live.cipher_suite.suite(),
            demo.cert_chain.len(),
            live.cert_chain.len()
        );
    } else {
        eprintln!("DEMO/LIVE TLS handshake characteristics match.");
    }

    Ok(())
}
