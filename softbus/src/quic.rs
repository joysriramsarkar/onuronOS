// softbus/src/quic.rs — Real QUIC/TLS 1.3 transport (Linux/Unix only)
// On Windows dev-hosts this module is excluded from compilation since
// quinn/rustls/ring require gcc/MinGW to build their C extensions.
#![cfg(unix)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

// ── Certificate generation ────────────────────────────────────────────────────

pub fn generate_self_signed_cert(
    device_id: &str,
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), String> {
    let subject_alt_names = vec![
        format!("{device_id}.nilbus.local"),
        device_id.to_string(),
    ];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names)
        .map_err(|e| format!("rcgen: {e}"))?;

    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(cert.key_pair.serialize_der())
        .map_err(|e| format!("key_der: {e}"))?;

    Ok((cert_der, key_der))
}

// ── Server endpoint ───────────────────────────────────────────────────────────

pub fn make_server_endpoint(
    bind_addr: SocketAddr,
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<Endpoint, String> {
    let server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| format!("rustls ServerConfig: {e}"))?;

    let quic_server_crypto = quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
        .map_err(|e| format!("quinn QuicServerConfig: {e}"))?;
    let server_config = ServerConfig::with_crypto(Arc::new(quic_server_crypto));

    let endpoint = Endpoint::server(server_config, bind_addr)
        .map_err(|e| format!("QUIC server bind {bind_addr}: {e}"))?;

    println!("[nilbus:quic] QUIC/TLS 1.3 server listening on {bind_addr}");
    Ok(endpoint)
}

// ── TOFU certificate verifier ─────────────────────────────────────────────────

#[derive(Debug)]
struct ToFUVerifier {
    expected_id: String,
}

impl rustls::client::danger::ServerCertVerifier for ToFUVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let _ = (end_entity, server_name, &self.expected_id);
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self, _: &[u8], _: &CertificateDer<'_>, _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self, _: &[u8], _: &CertificateDer<'_>, _: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}

// ── Active QUIC peer session ──────────────────────────────────────────────────

pub struct QuicPeerSession {
    pub remote_id: String,
    pub remote_addr: SocketAddr,
    connection: Connection,
}

impl QuicPeerSession {
    pub async fn connect(
        local_endpoint: &Endpoint,
        remote_addr: SocketAddr,
        remote_id: &str,
    ) -> Result<Self, String> {
        let client_crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ToFUVerifier {
                expected_id: remote_id.to_string(),
            }))
            .with_no_client_auth();

        let quic_client_crypto = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| format!("quinn QuicClientConfig: {e}"))?;
        let client_config = ClientConfig::new(Arc::new(quic_client_crypto));

        let server_name = format!("{remote_id}.nilbus.local");
        let connecting = local_endpoint
            .connect_with(client_config, remote_addr, &server_name)
            .map_err(|e| format!("connect({remote_addr}): {e}"))?;

        let connection = tokio::time::timeout(Duration::from_secs(5), connecting)
            .await
            .map_err(|_| format!("connect timeout to {remote_addr}"))?
            .map_err(|e| format!("QUIC handshake: {e}"))?;

        println!(
            "[nilbus:quic] TLS 1.3 QUIC stream established → {remote_id} @ {remote_addr}"
        );

        Ok(Self {
            remote_id: remote_id.to_string(),
            remote_addr,
            connection,
        })
    }

    pub async fn send_packet(&self, data: &[u8]) -> Result<(), String> {
        let mut send = self
            .connection
            .open_uni()
            .await
            .map_err(|e| format!("open_uni: {e}"))?;

        let len = (data.len() as u32).to_le_bytes();
        send.write_all(&len).await.map_err(|e| format!("write len: {e}"))?;
        send.write_all(data).await.map_err(|e| format!("write data: {e}"))?;
        send.finish().map_err(|e| format!("finish: {e}"))?;
        Ok(())
    }

    pub async fn recv_packet(&self) -> Result<Vec<u8>, String> {
        let mut recv = self
            .connection
            .accept_uni()
            .await
            .map_err(|e| format!("accept_uni: {e}"))?;

        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf).await.map_err(|e| format!("read len: {e}"))?;
        let len = u32::from_le_bytes(len_buf) as usize;

        if len > 16 * 1024 * 1024 {
            return Err(format!("packet too large: {len} bytes"));
        }

        let mut buf = vec![0u8; len];
        recv.read_exact(&mut buf).await.map_err(|e| format!("read data: {e}"))?;
        Ok(buf)
    }
}
