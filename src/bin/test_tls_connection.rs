//! TLS certificate validation test for cTrader API endpoints.
//!
//! Usage: cargo run --bin test-tls-connection

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use native_tls::TlsConnector as NativeTlsConnector;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tokio_native_tls::TlsConnector;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::{ParsedExtension, X509Certificate};

#[derive(Debug, Clone)]
struct CertInfo {
    subject: String,
    issuer: String,
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
    sans: Vec<String>,
}

#[derive(Debug, Clone)]
struct TlsResult {
    host: String,
    port: u16,
    cert: CertInfo,
    valid_now: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let targets = [
        ("live.ctraderapi.com", 5035),
        ("demo.ctraderapi.com", 5035),
    ];

    println!("=== TLS Certificate Validation (cTrader) ===");

    let mut results = Vec::new();
    for (host, port) in targets {
        println!("\n[TEST] {host}:{port}");
        match fetch_certificate(host, port).await {
            Ok(result) => {
                print_result(&result);
                results.push(result);
            }
            Err(err) => {
                println!("TLS handshake failed: {err}");
            }
        }
    }

    if results.len() == 2 {
        println!("\n=== Differences (LIVE vs DEMO) ===");
        print_differences(&results[0], &results[1]);
    } else {
        println!("\nNot enough results to compare certificates.");
    }

    Ok(())
}

async fn fetch_certificate(host: &str, port: u16) -> Result<TlsResult> {
    let addr = format!("{host}:{port}");
    let tcp = timeout(Duration::from_secs(10), TcpStream::connect(&addr))
        .await
        .context("tcp connect timeout")?
        .context("tcp connect failed")?;

    let connector = NativeTlsConnector::builder()
        .build()
        .context("failed to build TLS connector")?;
    let connector = TlsConnector::from(connector);

    let tls = timeout(Duration::from_secs(10), connector.connect(host, tcp))
        .await
        .context("TLS handshake timeout")?
        .context("TLS handshake failed")?;

    let peer_cert = tls
        .get_ref()
        .peer_certificate()
        .context("failed to read peer certificate")?
        .ok_or_else(|| anyhow!("no peer certificate presented"))?;

    let der = peer_cert.to_der().context("failed to serialize cert")?;
    let (_, cert) = X509Certificate::from_der(&der).map_err(|e| anyhow!("x509 parse error: {e}"))?;
    let cert_info = extract_cert_info(&cert)?;

    let now = Utc::now();
    let valid_now = now >= cert_info.not_before && now <= cert_info.not_after;

    Ok(TlsResult {
        host: host.to_string(),
        port,
        cert: cert_info,
        valid_now,
    })
}

fn extract_cert_info(cert: &X509Certificate<'_>) -> Result<CertInfo> {
    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();
    let not_before = cert
        .validity()
        .not_before
        .to_datetime()
        .context("invalid not_before")?;
    let not_after = cert
        .validity()
        .not_after
        .to_datetime()
        .context("invalid not_after")?;

    let mut sans = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::DNSName(dns) = name {
                    sans.push(dns.to_string());
                }
            }
        }
    }
    sans.sort();
    sans.dedup();

    Ok(CertInfo {
        subject,
        issuer,
        not_before,
        not_after,
        sans,
    })
}

fn print_result(result: &TlsResult) {
    println!("TLS handshake: OK");
    println!("Certificate subject: {}", result.cert.subject);
    println!("Certificate issuer: {}", result.cert.issuer);
    println!("Validity: {} -> {}", result.cert.not_before, result.cert.not_after);
    if result.cert.sans.is_empty() {
        println!("SANs: (none)");
    } else {
        println!("SANs: {}", result.cert.sans.join(", "));
    }
    println!("Valid now: {}", if result.valid_now { "YES" } else { "NO" });
}

fn print_differences(a: &TlsResult, b: &TlsResult) {
    let mut diffs = Vec::new();

    if a.cert.subject != b.cert.subject {
        diffs.push(format!("Subject differs:\n  LIVE: {}\n  DEMO: {}", a.cert.subject, b.cert.subject));
    }
    if a.cert.issuer != b.cert.issuer {
        diffs.push(format!("Issuer differs:\n  LIVE: {}\n  DEMO: {}", a.cert.issuer, b.cert.issuer));
    }
    if a.cert.not_before != b.cert.not_before || a.cert.not_after != b.cert.not_after {
        diffs.push(format!(
            "Validity differs:\n  LIVE: {} -> {}\n  DEMO: {} -> {}",
            a.cert.not_before, a.cert.not_after, b.cert.not_before, b.cert.not_after
        ));
    }
    if a.cert.sans != b.cert.sans {
        diffs.push(format!(
            "SANs differ:\n  LIVE: {}\n  DEMO: {}",
            if a.cert.sans.is_empty() {
                "(none)".to_string()
            } else {
                a.cert.sans.join(", ")
            },
            if b.cert.sans.is_empty() {
                "(none)".to_string()
            } else {
                b.cert.sans.join(", ")
            }
        ));
    }

    if diffs.is_empty() {
        println!("No certificate differences detected.");
    } else {
        for diff in diffs {
            println!("{diff}");
        }
    }
}
