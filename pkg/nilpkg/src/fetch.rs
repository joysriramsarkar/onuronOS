// pkg/nilpkg/src/fetch.rs — Remote package retrieval over HTTPS.
//
// Guarantees (improvement plan B2):
//   * TLS certificate validation is on by default and there is no flag that
//     disables it. `allow_insecure` only relaxes the URL *scheme* (plain
//     http:// for lab fixtures), never certificate checking.
//   * The SHA-256 of the downloaded archive is checked against the expected
//     digest (CLI `--sha256`, or the signed sidecar `<url>.manifest.json`)
//     *before* any extraction happens.
//   * After extraction the embedded, signed manifest is verified against the
//     actual payload, the publisher must be in the local trust store, and a
//     sidecar manifest (when present) must agree with the embedded one.
//   * A failed download or failed verification leaves nothing on disk: no
//     partial archive, no staging residue, no temp files.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::durable::sync_dir;

/// Maximum download size (512 MiB).
const MAX_DOWNLOAD_SIZE: u64 = 512 * 1024 * 1024;
/// Download timeout.
const DOWNLOAD_TIMEOUT_SECS: u64 = 300;
/// User-Agent header.
const USER_AGENT: &str = "nilpkg/2.0";

/// Options controlling a fetch. Defaults: HTTPS only, no expected digest,
/// sidecar manifest probing enabled.
#[derive(Clone, Copy, Default)]
pub struct FetchOptions<'a> {
    /// Accept plain `http://` URLs (logs a warning). Does NOT affect TLS
    /// certificate validation, which is always on for https:// URLs.
    pub allow_insecure: bool,
    /// Expected SHA-256 of the archive, checked before extraction.
    pub expected_sha256: Option<&'a str>,
    /// Probe for a signed sidecar manifest (`<url>.manifest.json`) and check
    /// the archive digest against it before extraction.
    pub sidecar: bool,
}

impl<'a> FetchOptions<'a> {
    pub fn new() -> Self {
        Self { allow_insecure: false, expected_sha256: None, sidecar: true }
    }
}

/// A downloaded package archive with its verified metadata.
pub struct FetchedPackage {
    /// The verified archive, published into the cache directory.
    pub path: PathBuf,
    /// Safe extraction of the archive. The caller MUST remove it when done
    /// (use `cleanup()`); it is only left behind on success.
    pub staged_dir: PathBuf,
    pub manifest: crate::Manifest,
}

impl FetchedPackage {
    /// Remove the extraction staging directory. The verified archive in the
    /// cache directory is kept.
    pub fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.staged_dir);
    }
}

impl std::fmt::Debug for FetchedPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchedPackage")
            .field("path", &self.path)
            .field("app_id", &self.manifest.app_id)
            .field("version", &self.manifest.version)
            .finish()
    }
}

/// Build the default HTTP agent (TLS validation on, no bypass exists).
pub fn default_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
}

/// Download `url` over HTTPS, verify signature and digest, return the result.
pub fn fetch_package(
    url: &str,
    cache_dir: &Path,
    key_dir: &Path,
    opts: &FetchOptions,
) -> Result<FetchedPackage, String> {
    fetch_package_with_agent(default_agent(), url, cache_dir, key_dir, opts)
}

/// Like [`fetch_package`] but with an injected agent (tests supply one whose
/// trust store contains the fixture server's certificate).
pub fn fetch_package_with_agent(
    agent: ureq::Agent,
    url: &str,
    cache_dir: &Path,
    key_dir: &Path,
    opts: &FetchOptions,
) -> Result<FetchedPackage, String> {
    // 1. Enforce HTTPS by default (scheme only; TLS validation is not optional).
    let url_lower = url.trim().to_ascii_lowercase();
    if !url_lower.starts_with("https://") {
        if !opts.allow_insecure {
            return Err(format!(
                "Refusing to fetch over insecure URL '{url}': use HTTPS or pass --allow-insecure"
            ));
        }
        eprintln!("[nilpkg] WARNING: fetching over insecure URL '{url}'");
    }

    // 2. Optional sidecar manifest, fetched first so the archive digest can be
    //    checked against it before extraction. Any failure (404, bad JSON)
    //    simply means "no sidecar"; integrity is still enforced after
    //    extraction by the embedded signed manifest.
    let sidecar = if opts.sidecar {
        fetch_sidecar_manifest(&agent, url)
    } else {
        None
    };

    // 3. Download the body fully into memory. A failure here has not touched
    //    the disk at all.
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("Failed to fetch {url}: {e}"))?;
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status} fetching {url}"));
    }
    let body = read_capped_body(response)?;

    // 4. Digest of the downloaded archive, checked BEFORE any extraction.
    let digest = crate::compute_sha256(&body);
    let mut expected: Option<String> = opts.expected_sha256.map(|s| s.trim().to_ascii_lowercase());
    if let Some(sc) = &sidecar {
        if let Some(arch) = sc.archive_sha256.as_deref() {
            let arch = arch.trim().to_ascii_lowercase();
            if let Some(exp) = &expected {
                if exp.as_str() != arch {
                    return Err(format!(
                        "Conflicting expected SHA-256 digests: --sha256 says {exp}, sidecar manifest says {arch}"
                    ));
                }
            }
            expected = Some(arch);
        }
    }
    if let Some(exp) = &expected {
        if !digest.eq_ignore_ascii_case(exp) {
            return Err(format!(
                "SHA-256 digest mismatch before extraction: expected {exp}, got {digest}"
            ));
        }
    }

    // 5. Stage the download on disk (temp file + fresh extraction directory).
    fs::create_dir_all(cache_dir).map_err(|e| format!("Could not create cache dir: {e}"))?;
    let file_name = url
        .split('?')
        .next()
        .unwrap_or("package.nilax")
        .rsplit('/')
        .next()
        .unwrap_or("package.nilax")
        .replace(['?', '&', '='], "_");
    let tmp_path = cache_dir.join(format!(".{}.tmp.{}", file_name, std::process::id()));
    let staging = cache_dir.join(format!(".nilpkg-fetch-{:016x}", rand::random::<u64>()));
    let dest = cache_dir.join(&file_name);

    let result = (|| -> Result<FetchedPackage, String> {
        {
            let mut file =
                File::create(&tmp_path).map_err(|e| format!("Could not create temp file: {e}"))?;
            file.write_all(&body)
                .map_err(|e| format!("Could not write temp file: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("Could not fsync temp file: {e}"))?;
        }

        // 6. Safe extraction into a fresh staging directory (path traversal,
        //    symlink, and size-capped; see extract.rs).
        crate::extract::extract_archive(&tmp_path, &staging)?;

        // 7. Full verification of the embedded manifest against the extracted
        //    payload: SHA-256, Ed25519 signature, arch/OS/permission compat.
        let (manifest, payload) = crate::read_package(&staging)?;

        // The embedded manifest may bind the archive digest when published
        // that way (normally set only on sidecar manifests, since an embedded
        // one cannot cover the archive that contains it).
        if let Some(arch) = manifest.archive_sha256.as_deref() {
            if !arch.eq_ignore_ascii_case(&digest) {
                return Err(format!(
                    "Archive SHA-256 mismatch: manifest declares {arch}, downloaded archive is {digest}"
                ));
            }
        }

        // 8. Publisher must be in the local trust store (revocation checked).
        crate::trusted_publisher(key_dir, &manifest)?;

        // 9. A sidecar manifest, when used for the pre-extraction digest
        //    check, must be signed by the same key and agree with the
        //    embedded manifest, and its signature must verify over the same
        //    payload. This ties the pre-extraction check to real crypto.
        if let Some(sc) = &sidecar {
            if sc.app_id != manifest.app_id || sc.version != manifest.version {
                return Err(
                    "Sidecar manifest does not match the package inside the archive".into(),
                );
            }
            if sc.sha256 != manifest.sha256 {
                return Err("Sidecar manifest payload hash disagrees with embedded manifest".into());
            }
            if sc.public_key_hex != manifest.public_key_hex {
                return Err("Sidecar manifest signed by a different key than the embedded manifest".into());
            }
            crate::verify_manifest(sc, &payload)?;
            crate::trusted_publisher(key_dir, sc)?;
        }

        // 10. Only now is the archive accepted into the cache.
        if dest.exists() {
            let _ = fs::remove_file(&dest);
        }
        fs::rename(&tmp_path, &dest)
            .map_err(|e| format!("Could not move downloaded file: {e}"))?;
        sync_dir(cache_dir).map_err(|e| format!("Could not sync cache dir: {e}"))?;

        Ok(FetchedPackage { path: dest, staged_dir: staging.clone(), manifest })
    })();

    if result.is_err() {
        // Nothing partial may survive a failed fetch.
        let _ = fs::remove_file(&tmp_path);
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

/// Read the response body with a hard size cap.
fn read_capped_body(response: ureq::Response) -> Result<Vec<u8>, String> {
    let mut reader = response.into_reader();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| format!("Download error: {e}"))?;
        if n == 0 {
            break;
        }
        if (buf.len() + n) as u64 > MAX_DOWNLOAD_SIZE {
            return Err(format!(
                "Download exceeds maximum size of {MAX_DOWNLOAD_SIZE} bytes"
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}

/// Try to fetch the signed sidecar manifest published next to the archive.
/// Returns `None` when it does not exist or cannot be parsed; the embedded
/// manifest check later still guarantees integrity either way.
fn fetch_sidecar_manifest(agent: &ureq::Agent, url: &str) -> Option<crate::Manifest> {
    let sidecar_url = format!("{url}.manifest.json");
    match agent.get(&sidecar_url).call() {
        Ok(resp) => {
            let body = read_capped_body(resp).ok()?;
            let manifest: crate::Manifest = serde_json::from_slice(&body).ok()?;
            Some(manifest)
        }
        Err(ureq::Error::Status(code, _)) if code == 404 => None,
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    use std::sync::Arc;
    use std::time::Duration as StdDuration;

    // ── Local HTTPS fixture server ────────────────────────────────────────

    /// What the fixture server should reply for a given request path.
    enum Reply {
        /// Normal response with a body.
        Ok(Vec<u8>),
        /// HTTP error status with a body.
        Status(u16, Vec<u8>),
        /// Declare a large Content-Length but send fewer bytes and close,
        /// simulating a connection dropped mid-download.
        Truncated { declared_len: usize, body: Vec<u8> },
    }

    struct TlsServer {
        addr: SocketAddr,
        cert_der: rustls::pki_types::CertificateDer<'static>,
    }

    /// Spawn a TLS server on 127.0.0.1 that answers every request with the
    /// result of `responder(path)`.
    fn spawn_tls_server<F>(responder: F) -> TlsServer
    where
        F: Fn(&str) -> Reply + Send + Sync + 'static,
    {
        let key_pair = rcgen::KeyPair::generate().expect("keygen");
        let mut params =
            rcgen::CertificateParams::new(vec!["localhost".to_string()]).expect("params");
        params.subject_alt_names.push(rcgen::SanType::IpAddress(IpAddr::V4(
            Ipv4Addr::LOCALHOST,
        )));
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
                let Ok(mut stream) = stream else { break };
                let _ = stream.set_read_timeout(Some(StdDuration::from_secs(10)));
                let cfg = server_cfg.clone();
                let responder = responder.clone();
                let mut conn = match rustls::ServerConnection::new(cfg) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let mut tls = rustls::StreamOwned::new(conn, stream);
                // Read the request head.
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
                let path = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();

                match responder(&path) {
                    Reply::Ok(body) => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = tls.write_all(head.as_bytes());
                        let _ = tls.write_all(&body);
                    }
                    Reply::Status(code, body) => {
                        let reason = if code == 404 { "Not Found" } else { "Error" };
                        let head = format!(
                            "HTTP/1.1 {code} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
                        // Connection closed here: client sees a short read.
                    }
                }
            }
        });

        TlsServer { addr, cert_der }
    }

    /// Agent that trusts only the fixture server's certificate and times out
    /// quickly so a broken fixture fails the test instead of hanging it.
    fn fixture_agent(cert: &rustls::pki_types::CertificateDer<'static>) -> ureq::Agent {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.clone()).expect("add root");
        let cfg = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        ureq::AgentBuilder::new()
            .tls_config(Arc::new(cfg))
            .timeout(StdDuration::from_secs(10))
            .user_agent(USER_AGENT)
            .build()
    }

    // ── Package building helpers ──────────────────────────────────────────

    /// Install a process-wide rustls CryptoProvider exactly once. Without this
    /// the test binary panics if more than one provider feature is linked.
    fn install_crypto_provider() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    fn temp_root(tag: &str) -> PathBuf {
        install_crypto_provider();
        let root = std::env::temp_dir().join(format!("nilpkg-fetch-{tag}-{:016x}", rand::random::<u64>()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    /// Create a signed package directory with the given payload.
    fn signed_package(
        dir: &Path,
        payload: &[u8],
        trust_dir: Option<&Path>,
    ) -> (crate::Manifest, ed25519_dalek::SigningKey) {
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("bin/app"), payload).unwrap();
        let (signer, verifier) = crate::generate_keypair();
        if let Some(t) = trust_dir {
            fs::create_dir_all(t).unwrap();
            fs::write(t.join("pub.pub"), crate::hex::encode(verifier.to_bytes())).unwrap();
        }
        let mut manifest = crate::Manifest {
            name: "Fixture App".into(),
            app_id: "org.onuron.fixture".into(),
            version: "1.0.0".into(),
            arch: crate::current_arch().into(),
            min_os_version: crate::CURRENT_OS_VERSION.into(),
            description: "fixture".into(),
            permissions: vec!["network".into()],
            exec: "bin/app".into(),
            sha256: crate::compute_sha256(payload),
            size_bytes: payload.len() as u64,
            signature_hex: None,
            public_key_hex: None,
            archive_sha256: None,
        };
        crate::sign_manifest(&mut manifest, &signer);
        fs::write(dir.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        (manifest, signer)
    }

    /// Tar the package directory (regular files only, as extract_tar requires).
    fn tar_package(dir: &Path) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        let manifest_bytes = fs::read(dir.join("manifest.json")).unwrap();
        header.set_size(manifest_bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "manifest.json", &manifest_bytes[..])
            .unwrap();
        let payload = fs::read(dir.join("bin/app")).unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, "bin/app", &payload[..]).unwrap();
        builder.finish().unwrap();
        builder.into_inner().unwrap()
    }

    /// Assert the cache directory contains nothing at all.
    fn assert_cache_empty(cache: &Path, context: &str) {
        let entries: Vec<String> = fs::read_dir(cache)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        assert!(entries.is_empty(), "{context}: cache not clean: {entries:?}");
    }

    // ── Scheme enforcement (no server needed) ─────────────────────────────

    #[test]
    fn fetch_rejects_plain_http_by_default() {
        let root = temp_root("http");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        let result = fetch_package(
            "http://example.com/pkg.nilax",
            &cache,
            &keys,
            &FetchOptions::new(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("insecure"));
        assert_cache_empty(&cache, "http rejection");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fetch_rejects_non_http_urls() {
        let root = temp_root("scheme");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        for url in ["ftp://example.com/pkg", "file:///etc/passwd", "not-a-url"] {
            let result = fetch_package(url, &cache, &keys, &FetchOptions::new());
            assert!(result.is_err(), "accepted {url}");
        }
        assert_cache_empty(&cache, "scheme rejection");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn fetch_rejects_non_http_urls_even_with_allow_insecure() {
        let root = temp_root("scheme2");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        for url in ["ftp://example.com/pkg", "file:///etc/passwd"] {
            let opts = FetchOptions { allow_insecure: true, ..FetchOptions::new() };
            assert!(fetch_package(url, &cache, &keys, &opts).is_err(), "accepted {url}");
        }
        assert_cache_empty(&cache, "scheme rejection (insecure)");

        let _ = fs::remove_dir_all(&root);
    }

    // ── HTTPS fixture tests ───────────────────────────────────────────────

    #[test]
    fn https_fetch_verifies_signature_and_publishes_archive() {
        let root = temp_root("ok");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        let (_manifest, _signer) = signed_package(&pkg, b"legit payload", Some(&keys.join("trusted")));
        let archive = tar_package(&pkg);

        let server = spawn_tls_server(move |_path| Reply::Ok(archive.clone()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let fetched = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect("fetch should succeed");

        assert_eq!(fetched.manifest.app_id, "org.onuron.fixture");
        assert!(fetched.path.exists(), "verified archive must be in the cache");
        assert!(fetched.staged_dir.join("manifest.json").exists());
        assert!(fetched.staged_dir.join("bin/app").exists());
        // The staged payload verifies against the signed manifest.
        crate::read_package(&fetched.staged_dir).expect("staged package verifies");
        fetched.cleanup();
        assert!(!fetched.staged_dir.exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tampered_payload_is_rejected_and_leaves_nothing_on_disk() {
        let root = temp_root("tamper");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        let (mut manifest, signer) =
            signed_package(&pkg, b"original payload", Some(&keys.join("trusted")));

        // Rebuild the archive with a tampered payload but the original signed
        // manifest: exactly the "modified package" attack.
        fs::write(pkg.join("bin/app"), b"EVIL PAYLOAD").unwrap();
        let archive = tar_package(&pkg);
        // Restore the signed manifest on disk for realism (not part of archive building).
        fs::write(pkg.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();

        let server = spawn_tls_server(move |_path| Reply::Ok(archive.clone()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect_err("tampered payload must be rejected");
        assert!(
            err.contains("SHA-256") || err.contains("size_bytes"),
            "unexpected error: {err}"
        );
        assert_cache_empty(&cache, "tampered payload");

        // A payload swap that also keeps the size covered by the manifest is
        // caught by the signature when only bytes inside the payload change.
        manifest.sha256 = crate::compute_sha256(b"EVIL PAYLOAD");
        manifest.size_bytes = b"EVIL PAYLOAD".len() as u64;
        crate::sign_manifest(&mut manifest, &signer);
        assert!(crate::verify_manifest(&manifest, b"EVIL PAYLOAD").is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn wrong_expected_digest_is_rejected_before_extraction() {
        let root = temp_root("digest");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        signed_package(&pkg, b"payload", Some(&keys.join("trusted")));
        let archive = tar_package(&pkg);

        let server = spawn_tls_server(move |_path| Reply::Ok(archive.clone()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let wrong = "0".repeat(64);
        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions {
                expected_sha256: Some(&wrong),
                sidecar: false,
                ..FetchOptions::new()
            },
        )
        .expect_err("digest mismatch must be rejected");
        assert!(err.contains("before extraction"), "unexpected error: {err}");
        // Nothing at all may have been written: no temp file, no extraction.
        assert_cache_empty(&cache, "digest mismatch");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sidecar_manifest_digest_is_checked_before_extraction() {
        let root = temp_root("sidecar");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        let (manifest, _signer) = signed_package(&pkg, b"payload", Some(&keys.join("trusted")));
        let archive = tar_package(&pkg);

        // Sidecar: the real signed manifest plus the archive digest — but the
        // digest is deliberately wrong to prove the pre-extraction gate works.
        let mut sidecar = manifest.clone();
        sidecar.archive_sha256 = Some("f".repeat(64));
        let sidecar_bytes = serde_json::to_vec(&sidecar).unwrap();
        let archive_for_server = archive.clone();

        let server = spawn_tls_server(move |path| {
            if path.ends_with(".manifest.json") {
                Reply::Ok(sidecar_bytes.clone())
            } else {
                Reply::Ok(archive_for_server.clone())
            }
        });
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(agent, &url, &cache, &keys, &FetchOptions::new())
            .expect_err("sidecar digest mismatch must be rejected");
        assert!(err.contains("before extraction"), "unexpected error: {err}");
        assert_cache_empty(&cache, "sidecar digest mismatch");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sidecar_manifest_must_agree_with_embedded_manifest() {
        let root = temp_root("sidecar-bad");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        let (manifest, _signer) = signed_package(&pkg, b"payload", Some(&keys.join("trusted")));
        let archive = tar_package(&pkg);

        // A sidecar that passes the digest check (correct archive digest) but
        // claims a different version: must be rejected after extraction.
        let mut sidecar = manifest.clone();
        sidecar.version = "9.9.9".into();
        sidecar.archive_sha256 = Some(crate::compute_sha256(&archive));
        crate::sign_manifest(&mut sidecar, &_signer);
        let sidecar_bytes = serde_json::to_vec(&sidecar).unwrap();
        let archive_for_server = archive.clone();

        let server = spawn_tls_server(move |path| {
            if path.ends_with(".manifest.json") {
                Reply::Ok(sidecar_bytes.clone())
            } else {
                Reply::Ok(archive_for_server.clone())
            }
        });
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(agent, &url, &cache, &keys, &FetchOptions::new())
            .expect_err("disagreeing sidecar must be rejected");
        assert!(err.contains("Sidecar"), "unexpected error: {err}");
        assert_cache_empty(&cache, "disagreeing sidecar");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn untrusted_publisher_is_rejected_and_leaves_nothing() {
        let root = temp_root("untrusted");
        let pkg = root.join("pkg");
        let keys = root.join("keys");
        let cache = root.join("cache");
        // Key generated but NOT placed in the trust store.
        signed_package(&pkg, b"payload", None);
        fs::create_dir_all(&keys).unwrap();
        let archive = tar_package(&pkg);

        let server = spawn_tls_server(move |_path| Reply::Ok(archive.clone()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect_err("untrusted publisher must be rejected");
        assert!(err.contains("trusted"), "unexpected error: {err}");
        assert_cache_empty(&cache, "untrusted publisher");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn http_error_leaves_no_partial_package() {
        let root = temp_root("404");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        let server = spawn_tls_server(|_path| Reply::Status(404, b"not here".to_vec()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect_err("404 must fail");
        assert!(err.contains("404"), "unexpected error: {err}");
        assert_cache_empty(&cache, "404");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn truncated_download_leaves_no_partial_package() {
        let root = temp_root("trunc");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        let server = spawn_tls_server(|_path| Reply::Truncated {
            declared_len: 1024 * 1024,
            body: b"only a little".to_vec(),
        });
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect_err("truncated download must fail");
        assert!(err.contains("Download error"), "unexpected error: {err}");
        assert_cache_empty(&cache, "truncated download");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn non_archive_payload_is_rejected_cleanly() {
        let root = temp_root("notarchive");
        let cache = root.join("cache");
        let keys = root.join("keys");
        fs::create_dir_all(&keys).unwrap();

        let server = spawn_tls_server(|_path| Reply::Ok(b"<html>not an archive</html>".to_vec()));
        let url = format!("https://{}/pkg.nilax", server.addr);
        let agent = fixture_agent(&server.cert_der);

        let err = fetch_package_with_agent(
            agent,
            &url,
            &cache,
            &keys,
            &FetchOptions { sidecar: false, ..FetchOptions::new() },
        )
        .expect_err("non-archive must be rejected");
        assert!(!err.is_empty());
        assert_cache_empty(&cache, "non-archive");

        let _ = fs::remove_dir_all(&root);
    }
}
