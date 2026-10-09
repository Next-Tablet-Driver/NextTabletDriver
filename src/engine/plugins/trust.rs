//! # Plugin Trust
//!
//! A plugin is a native library loaded into the driver's process: it runs with the full
//! rights of the app (and, through the engine thread, with access to every pen packet).
//! Loading whatever `.dll`/`.so` happens to sit in the plugins folder would let any
//! program that can write there get code executed the next time the driver starts.
//!
//! A library is therefore only loaded when it is **trusted**, i.e. one of:
//! - its SHA-256 is in the user's trust store (`trusted_plugins.json`), which is filled when
//!   the user installs a plugin through the app or explicitly approves one it found; or
//! - it ships with a valid minisign signature (`<library>.minisig`) from the key embedded at
//!   build time through `NTD_PLUGIN_PUBKEY` (used for first-party plugins).
//!
//! Anything else is reported as *untrusted* and left on disk, unloaded.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Minisign public key (base64, second line of the `.pub` file) of the official plugin
/// signing key. Provided by the release pipeline; absent in local builds.
const OFFICIAL_PUBKEY: Option<&str> = option_env!("NTD_PLUGIN_PUBKEY");

/// A plugin library found on disk that is not trusted and was therefore not loaded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UntrustedPlugin {
    /// File name of the library inside the plugins directory.
    pub file_name: String,
    /// Lowercase hex SHA-256 of the library; this is what the user approves.
    pub sha256: String,
}

#[derive(Default, Serialize, Deserialize)]
struct TrustFile {
    /// SHA-256 (lowercase hex) -> file name it was approved under (informational).
    trusted: BTreeMap<String, String>,
}

/// Persistent allow-list of approved plugin libraries, keyed by content hash.
pub struct TrustStore {
    path: PathBuf,
    entries: BTreeMap<String, String>,
}

impl TrustStore {
    /// Loads the store from `path`. A missing or unreadable file yields an empty store, which
    /// is the safe default: nothing is trusted until the user approves it.
    #[must_use]
    pub fn load(path: PathBuf) -> Self {
        let entries = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<TrustFile>(&text).ok())
            .map(|file| file.trusted)
            .unwrap_or_default();
        Self { path, entries }
    }

    /// Returns `true` if a library with this SHA-256 has been approved.
    #[must_use]
    pub fn is_trusted(&self, sha256: &str) -> bool {
        self.entries.contains_key(sha256)
    }

    /// Approves a library hash and persists the store.
    ///
    /// # Errors
    /// Returns an error string if the store cannot be written; the approval is then not kept.
    pub fn trust(&mut self, sha256: &str, file_name: &str) -> Result<(), String> {
        let previous = self
            .entries
            .insert(sha256.to_string(), file_name.to_string());
        self.save().inspect_err(|_| {
            // Do not keep an approval that could not be persisted.
            match previous {
                Some(ref name) => self.entries.insert(sha256.to_string(), name.clone()),
                None => self.entries.remove(sha256),
            };
        })
    }

    /// Forgets a library hash and persists the store.
    ///
    /// # Errors
    /// Returns an error string if the store cannot be written.
    pub fn revoke(&mut self, sha256: &str) -> Result<(), String> {
        if self.entries.remove(sha256).is_some() {
            self.save()?;
        }
        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&TrustFile {
            trusted: self.entries.clone(),
        })
        .map_err(|e| e.to_string())?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            e.to_string()
        })
    }
}

/// Computes the lowercase hex SHA-256 of a file, streaming it.
///
/// # Errors
/// Returns an I/O error if the file cannot be read.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if let Some(chunk) = buf.get(..n) {
            hasher.update(chunk);
        }
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Returns `true` if `library` has a `<library>.minisig` signature that verifies against the
/// official key. Always `false` when no official key was embedded at build time.
#[must_use]
pub fn has_valid_official_signature(library: &Path) -> bool {
    OFFICIAL_PUBKEY.is_some_and(|key| verify_signature(library, key))
}

fn verify_signature(library: &Path, pubkey_base64: &str) -> bool {
    let mut sig_path = library.as_os_str().to_owned();
    sig_path.push(".minisig");
    let Ok(sig_text) = fs::read_to_string(PathBuf::from(sig_path)) else {
        return false;
    };
    let Ok(public_key) = minisign_verify::PublicKey::from_base64(pubkey_base64.trim()) else {
        return false;
    };
    let Ok(signature) = minisign_verify::Signature::decode(&sig_text) else {
        return false;
    };
    let Ok(mut verifier) = public_key.verify_stream(&signature) else {
        return false;
    };
    let Ok(mut file) = fs::File::open(library) else {
        return false;
    };
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Some(chunk) = buf.get(..n) {
                    verifier.update(chunk);
                }
            }
            Err(_) => return false,
        }
    }
    verifier.finalize().is_ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!("ntd_trust_{name}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sha256_of_known_content() {
        let dir = temp_dir("sha");
        let file = dir.join("a.bin");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            sha256_file(&file).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn nothing_is_trusted_by_default() {
        let dir = temp_dir("empty");
        let store = TrustStore::load(dir.join("trusted_plugins.json"));
        assert!(!store.is_trusted("deadbeef"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn approvals_persist_and_can_be_revoked() {
        let dir = temp_dir("persist");
        let path = dir.join("trusted_plugins.json");

        let mut store = TrustStore::load(path.clone());
        store.trust("aa11", "plugin.dll").unwrap();
        assert!(store.is_trusted("aa11"));

        let reloaded = TrustStore::load(path.clone());
        assert!(reloaded.is_trusted("aa11"));
        assert!(!reloaded.is_trusted("bb22"));

        let mut store = reloaded;
        store.revoke("aa11").unwrap();
        assert!(!TrustStore::load(path).is_trusted("aa11"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_store_trusts_nothing() {
        let dir = temp_dir("corrupt");
        let path = dir.join("trusted_plugins.json");
        fs::write(&path, "{ definitely not json").unwrap();
        assert!(!TrustStore::load(path).is_trusted("aa11"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_or_bogus_signature_is_rejected() {
        let dir = temp_dir("sig");
        let lib = dir.join("p.dll");
        fs::write(&lib, b"binary").unwrap();
        // No .minisig next to it.
        assert!(!verify_signature(
            &lib,
            "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3"
        ));
        // A .minisig that is not a signature.
        fs::write(dir.join("p.dll.minisig"), "garbage").unwrap();
        assert!(!verify_signature(
            &lib,
            "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3"
        ));
        // The key embedded at build time is absent in tests, so nothing is "official".
        assert!(!has_valid_official_signature(&lib) || OFFICIAL_PUBKEY.is_some());
        let _ = fs::remove_dir_all(dir);
    }

    mod more {
        #![allow(clippy::indexing_slicing)]

        use super::*;

        const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

        #[test]
        fn trusting_a_library_persists_across_loads() {
            let dir = temp_dir("persist");
            let path = dir.join("trusted_plugins.json");
            let mut store = TrustStore::load(path.clone());
            assert!(!store.is_trusted(HASH));
            store.trust(HASH, "filter.dll").unwrap();
            assert!(store.is_trusted(HASH));
            assert!(TrustStore::load(path).is_trusted(HASH));
        }

        #[test]
        fn revoking_forgets_the_library_and_survives_a_reload() {
            let dir = temp_dir("revoke");
            let path = dir.join("trusted_plugins.json");
            let mut store = TrustStore::load(path.clone());
            store.trust(HASH, "filter.dll").unwrap();
            store.revoke(HASH).unwrap();
            assert!(!store.is_trusted(HASH));
            assert!(!TrustStore::load(path).is_trusted(HASH));
        }

        #[test]
        fn revoking_an_unknown_hash_writes_nothing() {
            let dir = temp_dir("revoke_unknown");
            let path = dir.join("trusted_plugins.json");
            let mut store = TrustStore::load(path.clone());
            store.revoke(HASH).unwrap();
            assert!(!path.exists());
        }

        #[test]
        fn an_unreadable_store_trusts_nothing() {
            let dir = temp_dir("garbage");
            let path = dir.join("trusted_plugins.json");
            fs::write(&path, "{ not json").unwrap();
            assert!(!TrustStore::load(path).is_trusted(HASH));
        }

        #[test]
        fn an_approval_that_cannot_be_saved_is_not_kept() {
            let dir = temp_dir("unsaveable");
            // The store's folder is a regular file, so it can never be created.
            let blocker = dir.join("blocker");
            fs::write(&blocker, "file").unwrap();
            let mut store = TrustStore::load(blocker.join("trusted_plugins.json"));
            assert!(store.trust(HASH, "filter.dll").is_err());
            assert!(!store.is_trusted(HASH));
        }

        #[test]
        fn a_failed_re_approval_restores_the_previous_file_name() {
            let dir = temp_dir("restore");
            let blocker = dir.join("blocker");
            fs::write(&blocker, "file").unwrap();
            let mut store = TrustStore {
                path: blocker.join("trusted_plugins.json"),
                entries: BTreeMap::from([(HASH.to_string(), "old.dll".to_string())]),
            };
            assert!(store.trust(HASH, "new.dll").is_err());
            assert_eq!(store.entries.get(HASH).map(String::as_str), Some("old.dll"));
        }

        #[test]
        fn a_store_path_that_cannot_be_replaced_reports_the_error_and_cleans_up() {
            let dir = temp_dir("rename_fail");
            // The store path is an existing directory: the final rename cannot succeed.
            let path = dir.join("store");
            fs::create_dir_all(path.join("child")).unwrap();
            let mut store = TrustStore::load(path.clone());
            assert!(store.trust(HASH, "filter.dll").is_err());
            assert!(!path.with_extension("json.tmp").exists());
        }

        #[test]
        fn sha256_of_a_missing_file_is_an_error() {
            let dir = temp_dir("sha_missing");
            assert!(sha256_file(&dir.join("nope.bin")).is_err());
        }

        #[test]
        fn sha256_streams_files_larger_than_one_buffer() {
            let dir = temp_dir("sha_big");
            let file = dir.join("big.bin");
            fs::write(&file, vec![b'a'; 200_000]).unwrap();
            let streamed = sha256_file(&file).unwrap();
            let mut hasher = Sha256::new();
            hasher.update(vec![b'a'; 200_000]);
            let expected: String = hasher.finalize().iter().fold(String::new(), |mut acc, b| {
                let _ = write!(acc, "{b:02x}");
                acc
            });
            assert_eq!(streamed, expected);
        }

        #[test]
        fn an_unsigned_or_wrongly_signed_library_is_never_official() {
            let dir = temp_dir("signature");
            let library = dir.join("filter.dll");
            fs::write(&library, b"test").unwrap();
            // Any key works to exercise the failure paths below.
            let key = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";

            // No signature file.
            assert!(!verify_signature(&library, key));
            // A signature file that is not a signature.
            fs::write(dir.join("filter.dll.minisig"), "garbage").unwrap();
            assert!(!verify_signature(&library, key));
            // A key that is not a key.
            assert!(!verify_signature(&library, "not base64 !"));
            // A missing library.
            assert!(!verify_signature(&dir.join("absent.dll"), key));
        }

        #[test]
        fn without_an_embedded_key_nothing_is_official() {
            // Local builds have no NTD_PLUGIN_PUBKEY: the check must fail closed.
            if OFFICIAL_PUBKEY.is_none() {
                let dir = temp_dir("no_key");
                let library = dir.join("filter.dll");
                fs::write(&library, b"test").unwrap();
                assert!(!has_valid_official_signature(&library));
            }
        }
    }
}
