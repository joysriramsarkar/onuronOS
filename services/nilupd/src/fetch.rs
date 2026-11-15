// services/nilupd/src/fetch.rs — Remote OTA update retrieval over HTTPS.
//
// Guarantees:
//   * TLS certificate validation is always enabled. `allow_insecure` only
//     relaxes the URL scheme check (plain http:// for local lab tests), never
//     certificate validation.
//   * The signed `update.json` manifest is downloaded and verified (Ed25519
//     signature + local trust store + not revoked) *before* streaming the
//     system image payload.
//   * The image payload is streamed directly to disk while hashing and checking
//     the byte count against `manifest.image_size` and `MAX_IMAGE_BYTES`, then
//     passed through `verify_image` before returning.
//   * Any failure removes the staging directory completely so no partial
//     update files remain on disk.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::{verify_image, verify_manifest_trust, UpdateManifest, MAX_IMAGE_BYTES, MAX_MANIFEST_BYTES};
use nilpkg::durable::{sync_dir, write_file_atomic};

const DOWNLOAD_TIMEOUT_SECS: u64 = 120;
const USER_AGENT: &str = concat!("nilupd/", env!("CARGO_PKG_VERSION"));

static STAGING_SEQ: AtomicU64 = AtomicU64::new(1);

/// Options governing remote update retrieval.
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchUpdateOptions {
    /// Allow plain `http://` URLs (scheme only; TLS cert validation is never disabled).
    pub allow_insecure: bool,
}

/// Verified update staged locally and ready for `apply_update`.
#[derive(Debug, Clone)]
pub struct FetchedUpdate {
    pub staging_dir: PathBuf,
    pub manifest: UpdateManifest,
}

/// Build the default HTTP agent (TLS certificate validation enabled).
pub fn default_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
}

fn validate_url(url: &str, allow_insecure: bool) -> Result<(), String> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("https://") {
        return Ok(());
    }
    if lower.starts_with("http://") {
        if allow_insecure {
            eprintln!("[nilupd] WARNING: fetching over insecure URL '{url}'");
            return Ok(());
        }
        return Err(format!(
            "Refusing to fetch over insecure URL '{url}': use HTTPS or pass --allow-insecure"
        ));
    }
    Err(format!("Unsupported URL scheme in '{url}': expected https://"))
}

/// Derive the image URL from a manifest URL when no explicit `--image` URL was given.
/// e.g. `https://updates.example/v2/update.json` -> `https://updates.example/v2/image`
pub fn derive_image_url(manifest_url: &str) -> String {
    let (base, query) = match manifest_url.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (manifest_url, None),
    };
    let derived = match base.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() && !dir.ends_with(":/") => format!("{dir}/image"),
        _ => format!("{base}.image"),
    };
    match query {
        Some(q) => format!("{derived}?{q}"),
        None => derived,
    }
}

fn read_bounded_bytes(response: ureq::Response, max_bytes: u64, label: &str) -> Result<Vec<u8>, String> {
    let mut reader = response.into_reader();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("Failed reading {label}: {e}"))?;
        if n == 0 {
            break;
        }
        if (buf.len() + n) as u64 > max_bytes {
            return Err(format!("{label} exceeds maximum size of {max_bytes} bytes"));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}

fn next_staging_dir(staging_root: &Path) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seq = STAGING_SEQ.fetch_add(1, Ordering::Relaxed);
    staging_root.join(format!(".nilupd-fetch-{}-{}-{seq}", std::process::id(), nanos))
}

/// Fetch a signed update manifest and image over HTTPS, verify signature,
/// publisher trust, byte length, and SHA-256, and stage it into a fresh
/// directory under `staging_root` ready for `apply_update`.
pub fn fetch_update(
    manifest_url: &str,
    image_url: Option<&str>,
    staging_root: &Path,
    key_dir: &Path,
    opts: &FetchUpdateOptions,
) -> Result<FetchedUpdate, String> {
    fetch_update_with_agent(default_agent(), manifest_url, image_url, staging_root, key_dir, opts)
}

/// Same as [`fetch_update`], using a caller-provided [`ureq::Agent`] (used in tests
/// to trust a local test CA/certificate without disabling validation).
pub fn fetch_update_with_agent(
    agent: ureq::Agent,
    manifest_url: &str,
    image_url: Option<&str>,
    staging_root: &Path,
    key_dir: &Path,
    opts: &FetchUpdateOptions,
) -> Result<FetchedUpdate, String> {
    validate_url(manifest_url, opts.allow_insecure)?;
    let img_url = image_url
        .map(|s| s.to_string())
        .unwrap_or_else(|| derive_image_url(manifest_url));
    validate_url(&img_url, opts.allow_insecure)?;

    // 1. Fetch and verify the signed manifest BEFORE streaming the large image.
    let manifest_resp = agent
        .get(manifest_url)
        .call()
        .map_err(|e| format!("Failed to fetch update manifest {manifest_url}: {e}"))?;
    let m_status = manifest_resp.status();
    if !(200..300).contains(&m_status) {
        return Err(format!("HTTP {m_status} fetching update manifest {manifest_url}"));
    }
    let manifest_bytes = read_bounded_bytes(manifest_resp, MAX_MANIFEST_BYTES, "Update manifest")?;
    let manifest: UpdateManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Invalid update manifest JSON: {e}"))?;
    verify_manifest_trust(&manifest, key_dir)?;

    if manifest.image_size > MAX_IMAGE_BYTES {
        return Err("Manifest image_size exceeds maximum allowed update size".into());
    }

    // 2. Prepare a fresh staging directory; cleaned up on any failure.
    fs::create_dir_all(staging_root)
        .map_err(|e| format!("Could not create update staging root: {e}"))?;
    let staging_dir = next_staging_dir(staging_root);
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Could not create staging dir {}: {e}", staging_dir.display()))?;

    let outcome = (|| -> Result<FetchedUpdate, String> {
        // 3. Stream the image payload to disk while computing SHA-256.
        let img_resp = agent
            .get(&img_url)
            .call()
            .map_err(|e| format!("Failed to fetch update image {img_url}: {e}"))?;
        let i_status = img_resp.status();
        if !(200..300).contains(&i_status) {
            return Err(format!("HTTP {i_status} fetching update image {img_url}"));
        }
        let declared_len: Option<u64> = img_resp
            .header("Content-Length")
            .and_then(|v| v.trim().parse::<u64>().ok());

        let part_path = staging_dir.join(".image.part");
        let image_path = staging_dir.join("image");
        let mut reader = img_resp.into_reader();
        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        {
            let mut file = File::create(&part_path)
                .map_err(|e| format!("Could not create staged image file: {e}"))?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = reader
                    .read(&mut buf)
                    .map_err(|e| format!("Download error reading update image: {e}"))?;
                if n == 0 {
                    break;
                }
                downloaded += n as u64;
                if downloaded > MAX_IMAGE_BYTES || downloaded > manifest.image_size {
                    return Err(format!(
                        "Update image stream exceeded declared size ({} bytes)",
                        manifest.image_size
                    ));
                }
                hasher.update(&buf[..n]);
                file.write_all(&buf[..n])
                    .map_err(|e| format!("Failed writing staged update image: {e}"))?;
            }
            file.sync_all()
                .map_err(|e| format!("Failed fsyncing staged update image: {e}"))?;
        }

        if let Some(cl) = declared_len {
            if downloaded != cl {
                return Err(format!(
                    "Truncated update image download: Content-Length {cl}, received {downloaded} bytes"
                ));
            }
        }
        if downloaded != manifest.image_size {
            return Err(format!(
                "Update image size mismatch: manifest declares {} bytes, downloaded {downloaded} bytes",
                manifest.image_size
            ));
        }
        let actual_sha = format!("{:064x}", hasher.finalize());
        if actual_sha != manifest.image_sha256 {
            return Err(format!(
                "Update image SHA-256 mismatch: expected {}, got {actual_sha}",
                manifest.image_sha256
            ));
        }

        fs::rename(&part_path, &image_path)
            .map_err(|e| format!("Failed finalizing staged image: {e}"))?;
        write_file_atomic(&staging_dir.join("update.json"), &manifest_bytes)?;
        sync_dir(&staging_dir)?;

        // 4. Final canonical verification pass over the staged directory.
        let verified = verify_image(&staging_dir, key_dir)?;
        Ok(FetchedUpdate {
            staging_dir: staging_dir.clone(),
            manifest: verified,
        })
    })();

    if outcome.is_err() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{apply_update, initialize_install_root, sign_update, status};
    use ed25519_dalek::SigningKey;
    use nilpkg::{compute_sha256, hex};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    use std::sync::Arc;

    struct TestDir(PathBuf);
    impl TestDir {
        fn new(label: &str) -> Self {
            let n = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let p = std::env::temp_dir().join(format!("nilupd-fetch-{label}-{}-{n}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    enum Reply {
        Ok(Vec<u8>),
        Status(u16, Vec<u8>),
        Truncated { declared_len: usize, body: Vec<u8> },
    }

    struct TlsServer {
        addr: SocketAddr,
        cert_der: rustls::pki_types::CertificateDer<'static>,
    }

    fn spawn_tls_server<F>(responder: F) -> TlsServer
    where
        F: Fn(&str) -> Reply + Send + Sync + 'static,
    {
        let key_pair = rcgen::KeyPair::generate().expect("keygen");
        let mut params =
            rcgen::CertificateParams::new(vec!["localhost".to_string()]).expect("params");
        params
            .subject_alt_names
            .push(rcgen::SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        let cert = params.self_signed(&key_pair).expect("self-sign");
        let cert_der = cert.der().clone();
        let key_der = key_pair.serialize_der();

        let server_cfg = Arc::new(
            rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert_der.clone()],
                    rustls::pki_types::PrivateKeyDer::Pkcs8(key_der.into()),
                )
                .expect("server config"),
        );

        let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).expect("bind");
        let addr = listener.local_addr().expect("addr");
        let responder = Arc::new(responder);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let cfg = server_cfg.clone();
                let responder = responder.clone();
                let conn = match rustls::ServerConnection::new(cfg) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let mut tls = rustls::StreamOwned::new(conn, stream);
                let mut req = Vec::new();
                let mut byte = [0u8; 1];
                let mut head_ok = false;
                while req.len() < 16 * 1024 {
                    match tls.read(&mut byte) {
                        Ok(0) => break,
                        Ok(_) => {
                            req.push(byte[0]);
                            if req.ends_with(b"\r\n\r\n") {
                                head_ok = true;
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                if !head_ok {
                    continue;
                }
                let request = String::from_utf8_lossy(&req);
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();

                match responder(&path) {
                    Reply::Ok(body) => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = tls.write_all(head.as_bytes());
                        let _ = tls.write_all(&body);
                    }
                    Reply::Status(code, body) => {
                        let head = format!(
                            "HTTP/1.1 {code} Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = tls.write_all(head.as_bytes());
                        let _ = tls.write_all(&body);
                    }
                    Reply::Truncated { declared_len, body } => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {declared_len}\r\nConnection: close\r\n\r\n"
                        );
                        let _ = tls.write_all(head.as_bytes());
                        let _ = tls.write_all(&body);
                    }
                }
            }
        });

        TlsServer { addr, cert_der }
    }

    fn test_agent(cert_der: rustls::pki_types::CertificateDer<'static>) -> ureq::Agent {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert_der).expect("add root");
        let tls = Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        );
        ureq::AgentBuilder::new()
            .tls_config(tls)
            .timeout(Duration::from_secs(10))
            .build()
    }

    fn trust_key(key_dir: &Path, name: &str, sk: &SigningKey) {
        let trusted = key_dir.join("trusted");
        fs::create_dir_all(&trusted).unwrap();
        let pub_hex = hex::encode(sk.verifying_key().to_bytes());
        fs::write(trusted.join(format!("{name}.pub")), pub_hex).unwrap();
    }

    fn signed_manifest_bytes(
        sk: &SigningKey,
        current_sha: &str,
        image: &[u8],
        target_slot: &str,
    ) -> Vec<u8> {
        let mut m = UpdateManifest {
            name: "onuron-ota".into(),
            version: "1.2.0".into(),
            target_slot: target_slot.into(),
            current_image_sha256: current_sha.into(),
            image_sha256: compute_sha256(image),
            image_size: image.len() as u64,
            target_device: String::new(),
            rollback_index: 1,
            signature_hex: None,
            public_key_hex: None,
        };
        sign_update(&mut m, sk);
        serde_json::to_vec(&m).unwrap()
    }

    #[test]
    fn fetch_rejects_plain_http_and_non_http_schemes() {
        let tmp = TestDir::new("schemes");
        let err = fetch_update(
            "http://updates.example/update.json",
            None,
            tmp.path(),
            tmp.path(),
            &FetchUpdateOptions::default(),
        )
        .unwrap_err();
        assert!(err.contains("insecure"));

        let err2 = fetch_update(
            "file:///tmp/update.json",
            None,
            tmp.path(),
            tmp.path(),
            &FetchUpdateOptions { allow_insecure: true },
        )
        .unwrap_err();
        assert!(err2.contains("Unsupported URL scheme"));
    }

    #[test]
    fn https_fetch_verifies_and_applies_ota_update() {
        let tmp = TestDir::new("ok");
        let root = tmp.path().join("sys");
        let keys = tmp.path().join("keys");
        let staging = tmp.path().join("staging");
        let old_img = b"onuron-slot-a-v1";
        let new_img = b"onuron-slot-b-v2-ota-payload";
        initialize_install_root(&root, old_img).unwrap();

        let sk = SigningKey::from_bytes(&[7u8; 32]);
        trust_key(&keys, "release", &sk);
        let manifest_json = signed_manifest_bytes(&sk, &compute_sha256(old_img), new_img, "B");
        let img_vec = new_img.to_vec();

        let srv = spawn_tls_server(move |path| match path {
            "/ota/update.json" => Reply::Ok(manifest_json.clone()),
            "/ota/image" => Reply::Ok(img_vec.clone()),
            _ => Reply::Status(404, b"not found".to_vec()),
        });
        let agent = test_agent(srv.cert_der);
        let url = format!("https://127.0.0.1:{}/ota/update.json", srv.addr.port());

        let fetched = fetch_update_with_agent(
            agent,
            &url,
            None,
            &staging,
            &keys,
            &FetchUpdateOptions::default(),
        )
        .expect("HTTPS fetch should succeed");

        assert_eq!(fetched.manifest.version, "1.2.0");
        apply_update(&fetched.staging_dir, &root, &keys).expect("apply fetched update");
        let st = status(&root).unwrap();
        assert_eq!(st.active_slot, "B");
        assert_eq!(st.running_image_sha256, compute_sha256(new_img));
    }

    #[test]
    fn tampered_image_or_untrusted_signer_leaves_no_staging_residue() {
        let tmp = TestDir::new("tamper");
        let keys = tmp.path().join("keys");
        let staging = tmp.path().join("staging");
        let sk_trusted = SigningKey::from_bytes(&[9u8; 32]);
        let sk_untrusted = SigningKey::from_bytes(&[10u8; 32]);
        trust_key(&keys, "release", &sk_trusted);

        let good_img = b"authentic-ota-image";
        let bad_img = b"tampered!-ota-image"; // same length, different bytes
        let m_good = signed_manifest_bytes(&sk_trusted, "0".repeat(64).as_str(), good_img, "B");
        let m_untrusted =
            signed_manifest_bytes(&sk_untrusted, "0".repeat(64).as_str(), good_img, "B");
        let bad_vec = bad_img.to_vec();
        let trunc_vec = good_img[..5].to_vec();
        let trunc_len = good_img.len();

        let srv = spawn_tls_server(move |path| match path {
            "/tamper/update.json" => Reply::Ok(m_good.clone()),
            "/tamper/image" => Reply::Ok(bad_vec.clone()),
            "/untrusted/update.json" => Reply::Ok(m_untrusted.clone()),
            "/untrusted/image" => Reply::Status(500, b"should not be requested".to_vec()),
            "/trunc/update.json" => Reply::Ok(m_good.clone()),
            "/trunc/image" => Reply::Truncated {
                declared_len: trunc_len,
                body: trunc_vec.clone(),
            },
            _ => Reply::Status(404, vec![]),
        });
        let agent = test_agent(srv.cert_der);

        // 1. Untrusted signer rejected before image download
        let u_url = format!("https://127.0.0.1:{}/untrusted/update.json", srv.addr.port());
        assert!(fetch_update_with_agent(
            agent.clone(),
            &u_url,
            None,
            &staging,
            &keys,
            &FetchUpdateOptions::default(),
        )
        .is_err());

        // 2. Tampered image rejected and staging cleaned up
        let t_url = format!("https://127.0.0.1:{}/tamper/update.json", srv.addr.port());
        let err = fetch_update_with_agent(
            agent.clone(),
            &t_url,
            None,
            &staging,
            &keys,
            &FetchUpdateOptions::default(),
        )
        .unwrap_err();
        assert!(err.contains("SHA-256 mismatch"));

        // 3. Truncated image rejected and staging cleaned up
        let tr_url = format!("https://127.0.0.1:{}/trunc/update.json", srv.addr.port());
        let err_tr = fetch_update_with_agent(
            agent,
            &tr_url,
            None,
            &staging,
            &keys,
            &FetchUpdateOptions::default(),
        )
        .unwrap_err();
        assert!(
            err_tr.contains("Truncated")
                || err_tr.contains("size mismatch")
                || err_tr.contains("Download error"),
            "unexpected error for truncated stream: {err_tr}"
        );

        if staging.exists() {
            assert_eq!(fs::read_dir(&staging).unwrap().count(), 0);
        }
    }
}
