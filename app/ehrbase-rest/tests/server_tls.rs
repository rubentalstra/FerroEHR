#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::expect_used,
    clippy::unwrap_used,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! `[server.tls]` — native TLS + the client-certificate gate (the IHE ATNA
//! ITI-19 node-authentication posture) on a real listener: an HTTPS client
//! trusting the test CA reaches `/rest/status` over TLS; with
//! `client_auth = "required"` a client presenting no certificate is rejected
//! at the handshake; with `"optional"` it is served. The accept-path for a
//! *valid* client certificate is rustls' `WebPkiClientVerifier` (upstream-
//! tested); the embedded test leaf is serverAuth-only, so it cannot double as
//! a client identity here.
//!
//! Uses the same offline-generated, 100-year test CA/leaf as the ATNA syslog
//! TLS round-trip test (CN=localhost, SAN localhost/127.0.0.1).

use std::io::Write;
use std::time::Duration;

use ehrbase::config::server::{ClientAuth, TlsConfig};

const TEST_CA_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----
MIIDFzCCAf+gAwIBAgIUAs65+YSFrH4uvtY9QWoTirKgkY8wDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPZWhyYmFzZS10ZXN0LWNhMCAXDTI2MDcwNjE3NTkwMVoY
DzIxMjYwNjEyMTc1OTAxWjAaMRgwFgYDVQQDDA9laHJiYXNlLXRlc3QtY2EwggEi
MA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC14PGEIvdnhTn4PjCWM6yzalPu
tB924PIMTCwTLadZ6yX+RSmzQkLHQu83TR8l2D1nSfJaQMrejT6BRIZGOmhjjsWK
D2ryY+m45ALweQYfPNstbrm9CQrAoNTWzgxA21ES3dH+2J5AKxDgQq1k2KqT5Lun
NLzxpMDVUYDvx3kMgakrdhvHuSXBP9bwu3+kWrKvlhl5JndHrj6ASciPuC291knI
r/g4u7aeCbFO+XH7+PsCPNsQuEC9ONMhEokMmGbqcVAHB6uOAM+El+mp0SbKUKfd
tWBEUd3IGCVIKe0OHc4H1faoyMx9fJ+F3fJR+77P5DlHnVapRcqj6RSNLLdlAgMB
AAGjUzBRMB0GA1UdDgQWBBSx14Rc1SngmufqoAwJU9BbIE5oxzAfBgNVHSMEGDAW
gBSx14Rc1SngmufqoAwJU9BbIE5oxzAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3
DQEBCwUAA4IBAQCtdVYkrqn7bx0Dy2GSJencQKyH8Ih7nXLUWQdCBprCrn39zWDV
69hnAfeEuMxobUCxwsdRt06wIxadfgawB0rJWZDlEJN5ERLfvHNPE6s1G/6tdRRP
bd6B2Pv4uI7IUdfH0zrpCtjLeQOZbrU5W6fuaGtUm9GxuwjPfuYOFjnNLojWGm54
FQ6i/VNDLJThupkO95ab44mY/S12Sxvm5WnIoDZFWXdxQ8t6vnKRX+m89CmyNKP2
9TLYW57BC6+RQlUJaXgqc+1t1vsWSJeAshGQcv2BxUTTvbMVhTBPlBUVy/XcgRCf
MmtVfj2FhFGZDZxzyYU7uc0sRB9ja6qBZ9hk
-----END CERTIFICATE-----
";

const TEST_LEAF_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----
MIIDSzCCAjOgAwIBAgIUZvcUkQeuzlUI70IpDWe6esK6Q1MwDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPZWhyYmFzZS10ZXN0LWNhMCAXDTI2MDcwNjE3NTkwMVoY
DzIxMjYwNjEyMTc1OTAxWjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwggEiMA0GCSqG
SIb3DQEBAQUAA4IBDwAwggEKAoIBAQCTDGF0kIaNYD3CNiO9XMir1OrsU/+IBtyX
6b9zfs7mMB3NSgi7AuBNdImg3WT4EwgQ/lyX9vOVgKuyiQAWQXMl1DXauY4+z1rr
N4ob6Lc2R4OLQEh6sqyoLMVwImafOkvXJ50rowWZKPiUBw437i4s1w1iaAf16ktw
kUu+1c8mfqzcnVdPJhWR2efU1KexWxsieNLICru3h5G3kVjkc3ulou5MpHBNKY6O
WnEmdtEdwn5seJCZfep+IAs5cAv38GJ505vTEf1J9Q6cgljk/EAeB58XCHo4imx+
pfgvoXBLyWR826IyB7YqMT8lTZbAWnoU5Yuag/BgZ0zI5MBvP6PzAgMBAAGjgYww
gYkwCQYDVR0TBAIwADAaBgNVHREEEzARgglsb2NhbGhvc3SHBH8AAAEwCwYDVR0P
BAQDAgWgMBMGA1UdJQQMMAoGCCsGAQUFBwMBMB0GA1UdDgQWBBT/CRm0D6LGZnW4
ZmpiK4+Yb53jszAfBgNVHSMEGDAWgBSx14Rc1SngmufqoAwJU9BbIE5oxzANBgkq
hkiG9w0BAQsFAAOCAQEAhMdKY/Qk5eJJhH0SsKQngd2EePo8/f3hWcucBWzCZ1zs
XTv1HoiJxPYJF9DIewF+x/TiEGow9WF8iU6EZT99w9IocIJHaFjFuFE1xmJlrlzv
5bkLIwNyrkJQzxURBzuvEwepPsF2K1RkJEqzk4d5Ffwxv6hkf+whfpXqXxwCO1PW
wQ/gmqFivCy8Zu99xO9ZYkQ/Gz5gCnna3snxutppF/xbWfQwOrq1HxgrAyIwFxAZ
CbLNvH16JKpegbWEN2llNt2kGdL62y9TPtNPr5v5prdEVnn7RUZvchABqdLYuyAW
P6DbgBmWaymqYHzOtsAx6zixYMWRkWJWMbq4mEp03Q==
-----END CERTIFICATE-----
";

const TEST_LEAF_KEY_PEM: &[u8] = b"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCTDGF0kIaNYD3C
NiO9XMir1OrsU/+IBtyX6b9zfs7mMB3NSgi7AuBNdImg3WT4EwgQ/lyX9vOVgKuy
iQAWQXMl1DXauY4+z1rrN4ob6Lc2R4OLQEh6sqyoLMVwImafOkvXJ50rowWZKPiU
Bw437i4s1w1iaAf16ktwkUu+1c8mfqzcnVdPJhWR2efU1KexWxsieNLICru3h5G3
kVjkc3ulou5MpHBNKY6OWnEmdtEdwn5seJCZfep+IAs5cAv38GJ505vTEf1J9Q6c
gljk/EAeB58XCHo4imx+pfgvoXBLyWR826IyB7YqMT8lTZbAWnoU5Yuag/BgZ0zI
5MBvP6PzAgMBAAECggEABBMyX5H2DYXXPxDdoofEybWhkojeTQmk31Kyz/wXLPSg
tPOIbSxzYoXNUB7Xc8agx13vAqZ3AK6yxJ01KRn1rxfDyV/FLTbhhzS4z4iSDG5C
TFg2oCJMXkhqar2QsinYLgpbdNGoLUo9hsyiBjzrV+Tr6n5eAEMRwL3q9g/3srTk
5NO/+mAd2ODzrz6SwWbZSQpQZ/rvY4bmCihLCl3KzXR53M4c2Q6RZOH4Y1Dk3t1X
Rk5qag+88SU3d0y7S6HZkS66psWlBGoAm13Nlp+BjUdw04pCNbtH98XEZtHIhHDu
dlG96/c9WQIalB24Do4MD7/4dpc4jlBKHRCNtcPm4QKBgQDNM7TvScNNPtOtQIfT
Zj1neBu18Ol1QgdSw1f5uSgk7dlUD4ZyARXVQE/wBhdou4Ly2K41MkkQbpMai84T
hhdgaoK8hq7uM7OeAueduEg5tZYGPOXvCi6oDxT2u0XiZmxCBR2NnPG+S9PNX6Th
dFUz2VGsYN1e8LKqFNnIVBDOKwKBgQC3c0v13xNWh3TKPcJox3PkeVhreYTQnmGk
tNVL+JWIybcBbxszkOGnKpDoK1EXLt3aY7wz/3vceTPmLLRzF119locszvVWUPeK
803JJS0BuLx2iYGbE/7Ms4uPSIE1GsnTh2R7/9q7nMcn0IDY3ccDtTVV5am5cnLu
wmnHRLNlWQKBgQDFq/W0PP5iPw9yaiNtxaOJNO6cycJbLowXcg4hhffh+y3MCFif
IeSCVT2sHnIWdeujPJA0togjyCD1BZAyxo1mV7QxIB0LYcq5gFrWWFtbHE21HJzz
NH/VbRHozGZ5veizgHIDpRhLFIin4mbWuFYLaGGBCJ63gm7Z4NQaYsD9mQKBgCkT
RcTNp/TN65aE8YyobAiSKvwVf2l+rPpGCyQxirnQAIvvprWLERtu6ncxi7yXH9GQ
V0mFQOSCtt4o0FacurAuDiI4TLKA+oxAIgCRtFwYUwpvi3d/qKOI6Ayy/Us4rkwQ
mq0xAnbTibwecVsdfTwVTNXo0HHXcGiJW0nk1g4xAoGAWRcc/XQTYPmgw3QtkyfA
z12RhYxzUUaiiGwYTbSyitIKvySud+vCPw7Mnz1cdC63XOUN/mQr2Nwht2oFtLvl
s29Z18H0uXzhfKQVu9kMRzdods9GZnHuEkPgQAO21av2ermD0646zvNsMZCAXPo1
F9DV18ILppscP5wy0SP20/U=
-----END PRIVATE KEY-----
";

/// Write the PEMs to temp files and build the `[server.tls]` section.
fn tls_files(dir: &assert_fs::TempDir, client_auth: ClientAuth) -> TlsConfig {
    let write = |name: &str, bytes: &[u8]| {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).expect("create pem");
        f.write_all(bytes).expect("write pem");
        path.to_string_lossy().into_owned()
    };
    TlsConfig {
        enabled: true,
        cert_file: Some(write("server.pem", TEST_LEAF_PEM)),
        key_file: Some(write("server.key", TEST_LEAF_KEY_PEM)),
        client_auth,
        client_ca_file: match client_auth {
            ClientAuth::Off => None,
            _ => Some(write("client-ca.pem", TEST_CA_PEM)),
        },
    }
}

/// Serve a minimal router over `[server.tls]` on an ephemeral port via the
/// crate's real TLS path; return the bound port.
async fn serve_tls(tls: TlsConfig) -> u16 {
    let app = axum::Router::new().route("/rest/status", axum::routing::get(|| async { "OK" }));
    // Bind an ephemeral port first so the test knows where to connect.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let config = ehrbase_rest::tls_server_config(&tls).expect("tls config");
    tokio::spawn(async move {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        axum_server::bind_rustls(
            addr,
            axum_server::tls_rustls::RustlsConfig::from_config(config),
        )
        .serve(app.into_make_service())
        .await
        .ok();
    });
    // Wait for the listener to come up.
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("TLS listener did not come up");
}

fn https_client() -> reqwest::Client {
    // A preconfigured rustls config trusting ONLY the test CA: the platform
    // verifier (reqwest's default) rejects the 100-year offline test leaf.
    use rustls::pki_types::pem::PemObject;
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls::pki_types::CertificateDer::pem_slice_iter(TEST_CA_PEM) {
        roots.add(cert.expect("ca cert")).expect("add root");
    }
    let provider = std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
}

#[tokio::test]
async fn serves_https_with_the_configured_certificate() {
    let dir = assert_fs::TempDir::new().expect("tempdir");
    let port = serve_tls(tls_files(&dir, ClientAuth::Off)).await;
    let resp = https_client()
        .get(format!("https://localhost:{port}/rest/status"))
        .send()
        .await
        .expect("https request");
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn required_client_auth_rejects_certificateless_clients() {
    let dir = assert_fs::TempDir::new().expect("tempdir");
    let port = serve_tls(tls_files(&dir, ClientAuth::Required)).await;
    // No client certificate → the handshake is rejected (IHE ATNA ITI-19:
    // mutually authenticated nodes; a bare TLS client never reaches HTTP).
    let outcome = https_client()
        .get(format!("https://localhost:{port}/rest/status"))
        .send()
        .await;
    assert!(
        outcome.is_err(),
        "a certificate-less client must be rejected: {outcome:?}"
    );
}

#[tokio::test]
async fn optional_client_auth_still_serves_certificateless_clients() {
    let dir = assert_fs::TempDir::new().expect("tempdir");
    let port = serve_tls(tls_files(&dir, ClientAuth::Optional)).await;
    let resp = https_client()
        .get(format!("https://localhost:{port}/rest/status"))
        .send()
        .await
        .expect("https request");
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn missing_key_material_is_a_typed_boot_error() {
    let tls = TlsConfig {
        enabled: true,
        ..TlsConfig::default()
    };
    let err = ehrbase_rest::tls_server_config(&tls).expect_err("must fail");
    assert!(err.to_string().contains("server.tls.cert_file"));
}
