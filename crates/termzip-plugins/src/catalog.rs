//! The plugin catalog: a list of plugins somebody published, signed so that nothing in it is trusted unless it
//! came from the holder of the catalog key.
//!
//! Network is used only when the user asks (loading the catalog, installing a plugin). The order is always the
//! same: fetch the index **and its minisign signature**, verify the signature against the public key that is
//! built into TermZip, and only then read the index. A package is then fetched over HTTPS, and must have the
//! size and the SHA-256 the signed index promises, carry a manifest with the same id, version and permissions,
//! and pass every check of a local file. What the user sees before installing is therefore what they get.

use std::collections::HashSet;
use std::time::Duration;

use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use termzip_plugin_api::{MAX_NAME_CHARS, MAX_PLUGIN_BYTES, Permission};

use crate::manifest::read_module;
use crate::store::sha256_hex;

pub const MAX_INDEX_BYTES: usize = 256 * 1024;
pub const MAX_CATALOG_PLUGINS: usize = 128;
const MAX_SIGNATURE_BYTES: usize = 4 * 1024;
const MAX_TEXT_CHARS: usize = 280;
const MAX_URL_CHARS: usize = 512;
/// The format of `index.json` this TermZip reads.
pub const INDEX_FORMAT: u32 = 1;

/// One plugin the catalog offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    /// What the plugin asks for, shown before it is installed and checked against the file afterwards.
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Where the `.wasm` file is, over HTTPS.
    pub url: String,
    /// The SHA-256 of the file, in lowercase hex.
    pub sha256: String,
    pub size: u64,
}

/// What a catalog offered: the usable entries, and a sentence for each one that was left out.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Catalog {
    pub plugins: Vec<CatalogEntry>,
    pub problems: Vec<String>,
}

/// Where the catalog is and the key it is signed with.
#[derive(Debug, Clone, Copy)]
pub struct CatalogSource<'a> {
    /// The address of `index.json`; its signature is at the same address with `.minisig` added.
    pub index_url: &'a str,
    /// The public key as minisign writes it (`RWQ...`).
    pub public_key: &'a str,
}

/// Something that can fetch an address. The real one speaks HTTPS; tests use a fake.
pub trait Fetch {
    /// The body at `url`, at most `limit` bytes; more is an error.
    fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String>;
}

/// Fetching over HTTPS with limits on time, size and redirects.
pub struct HttpFetch {
    agent: ureq::Agent,
}

impl HttpFetch {
    pub fn new() -> Self {
        Self::build(true, Duration::from_secs(30))
    }

    fn build(https_only: bool, timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(https_only)
            .max_redirects(3)
            .timeout_global(Some(timeout))
            .user_agent(concat!("TermZip/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            agent: config.into(),
        }
    }

    /// For tests against a server on the loopback address, which has no certificate.
    #[cfg(test)]
    fn insecure_for_tests(timeout: Duration) -> Self {
        Self::build(false, timeout)
    }

    /// Like [`HttpFetch::new`] but allowing plain http, for trying a catalog served from this machine. It
    /// does not exist in a release build.
    #[cfg(debug_assertions)]
    pub fn for_development() -> Self {
        Self::build(false, Duration::from_secs(30))
    }
}

impl Default for HttpFetch {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetch for HttpFetch {
    fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String> {
        let mut response = self.agent.get(url).call().map_err(|error| match error {
            ureq::Error::StatusCode(code) => format!("The server answered {code}."),
            ureq::Error::Timeout(_) => "The server took too long to answer.".to_owned(),
            ureq::Error::Io(error) => format!("Could not reach the server: {error}."),
            other => format!("Could not fetch it: {other}."),
        })?;
        // The reader refuses a body that reaches its limit, so it is given one byte more than is allowed, and
        // an answer of exactly `limit` bytes (what `download` asks for) is read like any other.
        let body = response
            .body_mut()
            .with_config()
            .limit(limit as u64 + 1)
            .read_to_vec()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => too_large(limit),
                other => format!("Could not read the answer: {other}."),
            })?;
        if body.len() > limit {
            return Err(too_large(limit));
        }
        Ok(body)
    }
}

fn too_large(limit: usize) -> String {
    if limit >= 1024 {
        format!("The answer is larger than {} KiB.", limit / 1024)
    } else {
        format!("The answer is larger than {limit} bytes.")
    }
}

fn is_hex_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_slug(text: &str) -> bool {
    !text.is_empty()
        && text.chars().count() <= MAX_NAME_CHARS
        && text.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// Where TermZip may fetch from: https, always. A debug build (never a release one) also allows a server on
/// this machine, so the catalog can be tried without a certificate; the signature is checked all the same.
fn https(url: &str) -> bool {
    let local = cfg!(debug_assertions)
        && (url.starts_with("http://127.0.0.1:") || url.starts_with("http://localhost:"));
    (url.starts_with("https://") || local)
        && url.chars().count() <= MAX_URL_CHARS
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Checks one entry of the index.
pub fn validate_entry(entry: &CatalogEntry) -> Result<(), String> {
    if !is_slug(&entry.id) {
        return Err(format!("the id “{}” is not valid", entry.id));
    }
    if entry.name.trim().is_empty() || entry.name.chars().count() > MAX_NAME_CHARS {
        return Err("the name is empty or too long".to_owned());
    }
    if semver::Version::parse(&entry.version).is_err() {
        return Err(format!("the version “{}” is not valid", entry.version));
    }
    for text in [
        Some(&entry.description),
        entry.author.as_ref(),
        entry.license.as_ref(),
        entry.homepage.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err("a text in it is too long".to_owned());
        }
    }
    if let Some(homepage) = &entry.homepage
        && !(homepage.starts_with("https://") || homepage.starts_with("http://"))
    {
        return Err("the homepage is not a web address".to_owned());
    }
    if !https(&entry.url) {
        return Err("the download address is not an https address".to_owned());
    }
    if !is_hex_digest(&entry.sha256) {
        return Err("the checksum is not a SHA-256 in lowercase hex".to_owned());
    }
    if entry.size == 0 || entry.size > MAX_PLUGIN_BYTES as u64 {
        return Err("the size is not between 1 byte and 4 MiB".to_owned());
    }
    let mut seen = HashSet::new();
    if !entry
        .permissions
        .iter()
        .all(|permission| seen.insert(*permission))
    {
        return Err("a permission is listed twice".to_owned());
    }
    Ok(())
}

/// Reads an index that has **already been verified**. An entry that is not usable is left out with a sentence;
/// the first of two entries with the same id wins.
pub fn parse_index(bytes: &[u8]) -> Result<Catalog, String> {
    #[derive(Deserialize)]
    struct Raw {
        version: u32,
        #[serde(default)]
        plugins: Vec<serde_json::Value>,
    }
    let raw: Raw = serde_json::from_slice(bytes)
        .map_err(|error| format!("The catalog is not readable: {error}."))?;
    if raw.version != INDEX_FORMAT {
        return Err(format!(
            "The catalog uses format {}, which this TermZip does not read (it reads format {INDEX_FORMAT}). Update TermZip.",
            raw.version
        ));
    }
    let mut catalog = Catalog::default();
    let mut ids = HashSet::new();
    for (position, value) in raw.plugins.into_iter().enumerate() {
        if catalog.plugins.len() >= MAX_CATALOG_PLUGINS {
            catalog.problems.push(format!(
                "Only the first {MAX_CATALOG_PLUGINS} plugins of the catalog are listed."
            ));
            break;
        }
        let entry = match serde_json::from_value::<CatalogEntry>(value) {
            Ok(entry) => entry,
            Err(error) => {
                catalog.problems.push(format!(
                    "Entry {} of the catalog was left out: {error}.",
                    position + 1
                ));
                continue;
            }
        };
        if let Err(why) = validate_entry(&entry) {
            catalog.problems.push(format!(
                "“{}” was left out of the catalog: {why}.",
                entry.id
            ));
        } else if !ids.insert(entry.id.clone()) {
            catalog.problems.push(format!(
                "“{}” is listed twice in the catalog; the first entry is used.",
                entry.id
            ));
        } else {
            catalog.plugins.push(entry);
        }
    }
    Ok(catalog)
}

/// Checks the signature of an index against the catalog key and only then reads it.
pub fn verify_and_parse(
    index: &[u8],
    signature: &str,
    public_key: &str,
) -> Result<Catalog, String> {
    let key = PublicKey::from_base64(public_key)
        .map_err(|_| "The catalog key built into this TermZip is not usable.".to_owned())?;
    let signature = Signature::decode(signature)
        .map_err(|_| "The catalog's signature file is not a signature.".to_owned())?;
    key.verify(index, &signature, false).map_err(|_| {
        "The catalog's signature does not match: it was not signed with TermZip's catalog key, or it was changed on its way. Nothing in it was used."
            .to_owned()
    })?;
    parse_index(index)
}

/// Fetches the catalog and its signature, verifies, and reads it.
pub fn load_catalog(fetch: &dyn Fetch, source: CatalogSource<'_>) -> Result<Catalog, String> {
    if !https(source.index_url) {
        return Err("The catalog address is not an https address.".to_owned());
    }
    let index = fetch
        .get(source.index_url, MAX_INDEX_BYTES)
        .map_err(|error| format!("Could not load the catalog: {error}"))?;
    let signature = fetch
        .get(
            &format!("{}.minisig", source.index_url),
            MAX_SIGNATURE_BYTES,
        )
        .map_err(|error| format!("Could not load the catalog's signature: {error}"))?;
    let signature = String::from_utf8(signature)
        .map_err(|_| "The catalog's signature file is not text.".to_owned())?;
    verify_and_parse(&index, &signature, source.public_key)
}

/// Downloads a plugin and checks that it is the plugin the signed index described.
pub fn download(fetch: &dyn Fetch, entry: &CatalogEntry) -> Result<Vec<u8>, String> {
    validate_entry(entry).map_err(|why| format!("That catalog entry cannot be used: {why}."))?;
    let bytes = fetch
        .get(&entry.url, entry.size as usize)
        .map_err(|error| format!("Could not download “{}”: {error}", entry.name))?;
    if bytes.len() as u64 != entry.size {
        return Err(format!(
            "“{}” is not the size the catalog promised.",
            entry.name
        ));
    }
    if sha256_hex(&bytes) != entry.sha256 {
        return Err(format!(
            "“{}” does not match the checksum in the catalog, so it was not installed.",
            entry.name
        ));
    }
    let module = read_module(&bytes)?;
    let manifest = module.manifest;
    if manifest.id != entry.id || manifest.version != entry.version {
        return Err(format!(
            "“{}” says it is {} {}, not what the catalog listed, so it was not installed.",
            entry.name, manifest.id, manifest.version
        ));
    }
    let mut promised = entry.permissions.clone();
    let mut asked = manifest.permissions.clone();
    promised.sort();
    asked.sort();
    if promised != asked {
        return Err(format!(
            "“{}” asks for other permissions than the catalog showed, so it was not installed.",
            entry.name
        ));
    }
    Ok(bytes)
}

/// Whether the catalog has a newer version than the installed one.
pub fn is_newer(installed: &str, offered: &str) -> bool {
    match (
        semver::Version::parse(installed),
        semver::Version::parse(offered),
    ) {
        (Ok(installed), Ok(offered)) => offered > installed,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::fixtures::{MINIMAL, minimal, plugin};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // --- signing, with a key made for the test ------------------------------------------------

    struct Keys {
        public: String,
        secret: minisign::SecretKey,
        public_key: minisign::PublicKey,
    }

    fn keys() -> Keys {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        Keys {
            public: pair.pk.to_base64(),
            secret: pair.sk,
            public_key: pair.pk,
        }
    }

    fn sign(keys: &Keys, data: &[u8]) -> String {
        minisign::sign(Some(&keys.public_key), &keys.secret, data, None, None)
            .unwrap()
            .into_string()
    }

    fn entry_for(bytes: &[u8], id: &str, version: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.to_owned(),
            name: "Hello".to_owned(),
            version: version.to_owned(),
            description: "Says hello".to_owned(),
            author: Some("Someone".to_owned()),
            license: Some("MIT".to_owned()),
            homepage: Some("https://example.com".to_owned()),
            permissions: vec![Permission::Blocks],
            url: format!("https://plugins.example.com/{id}.wasm"),
            sha256: sha256_hex(bytes),
            size: bytes.len() as u64,
        }
    }

    fn index_with(entries: &[CatalogEntry]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({ "version": 1, "plugins": entries })).unwrap()
    }

    struct Fake(HashMap<String, Vec<u8>>);

    impl Fetch for Fake {
        fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String> {
            let body = self
                .0
                .get(url)
                .ok_or_else(|| "The server answered 404.".to_owned())?;
            if body.len() > limit {
                return Err(format!("The answer is larger than {} KiB.", limit / 1024));
            }
            Ok(body.clone())
        }
    }

    const INDEX_URL: &str = "https://plugins.example.com/index.json";

    fn published(keys: &Keys, entries: &[CatalogEntry]) -> Fake {
        let index = index_with(entries);
        let signature = sign(keys, &index);
        Fake(HashMap::from([
            (INDEX_URL.to_owned(), index),
            (format!("{INDEX_URL}.minisig"), signature.into_bytes()),
        ]))
    }

    #[test]
    fn a_signed_catalog_is_read() {
        let keys = keys();
        let hello = entry_for(&minimal(), "hello", "1.0.0");
        let fetch = published(&keys, std::slice::from_ref(&hello));

        let catalog = load_catalog(
            &fetch,
            CatalogSource {
                index_url: INDEX_URL,
                public_key: &keys.public,
            },
        )
        .unwrap();

        assert_eq!(catalog.plugins, [hello]);
        assert!(catalog.problems.is_empty());
    }

    #[test]
    fn nothing_is_read_from_a_catalog_whose_signature_does_not_match() {
        let keys = keys();
        let hello = entry_for(&minimal(), "hello", "1.0.0");
        let index = index_with(&[hello]);
        let signature = sign(&keys, &index);

        // A changed index: one byte.
        let mut changed = index.clone();
        changed[20] ^= 1;
        let error = verify_and_parse(&changed, &signature, &keys.public).unwrap_err();
        assert!(error.contains("signature does not match"), "{error}");

        // Signed by somebody else.
        let other = self::keys();
        let forged = sign(&other, &index);
        assert!(
            verify_and_parse(&index, &forged, &keys.public)
                .unwrap_err()
                .contains("does not match")
        );

        // Not a signature at all, or a key that is not a key.
        assert!(
            verify_and_parse(&index, "garbage", &keys.public)
                .unwrap_err()
                .contains("not a signature")
        );
        assert!(
            verify_and_parse(&index, &signature, "not a key")
                .unwrap_err()
                .contains("key built into")
        );
        // The signature of one file does not do for another.
        let another = index_with(&[]);
        assert!(verify_and_parse(&another, &signature, &keys.public).is_err());
    }

    #[test]
    fn a_catalog_without_its_signature_is_refused() {
        let keys = keys();
        let mut fetch = published(&keys, &[]);
        fetch.0.remove(&format!("{INDEX_URL}.minisig"));

        let error = load_catalog(
            &fetch,
            CatalogSource {
                index_url: INDEX_URL,
                public_key: &keys.public,
            },
        )
        .unwrap_err();
        assert!(error.contains("signature"), "{error}");
        let error = load_catalog(
            &fetch,
            CatalogSource {
                index_url: "http://plugins.example.com/index.json",
                public_key: &keys.public,
            },
        )
        .unwrap_err();
        assert!(error.contains("https"), "{error}");
        // A catalog that cannot be reached says so.
        let error = load_catalog(
            &Fake(HashMap::new()),
            CatalogSource {
                index_url: INDEX_URL,
                public_key: &keys.public,
            },
        )
        .unwrap_err();
        assert!(error.starts_with("Could not load the catalog"), "{error}");
    }

    #[test]
    fn entries_that_cannot_be_used_are_left_out_and_explained() {
        let good = entry_for(&minimal(), "hello", "1.0.0");
        let mut bad_url = entry_for(&minimal(), "plain", "1.0.0");
        bad_url.url = "http://plugins.example.com/plain.wasm".to_owned();
        let mut bad_sum = entry_for(&minimal(), "sum", "1.0.0");
        bad_sum.sha256 = "ABC".to_owned();
        let mut too_big = entry_for(&minimal(), "big", "1.0.0");
        too_big.size = MAX_PLUGIN_BYTES as u64 + 1;
        let mut bad_version = entry_for(&minimal(), "ver", "1.0.0");
        bad_version.version = "one".to_owned();
        let mut bad_id = entry_for(&minimal(), "Bad Id", "1.0.0");
        bad_id.id = "Bad Id".to_owned();
        let duplicate = entry_for(&minimal(), "hello", "2.0.0");

        let mut index = serde_json::json!({
            "version": 1,
            "plugins": [good, bad_url, bad_sum, too_big, bad_version, bad_id, duplicate, { "id": "no-fields" }],
        });
        let catalog = parse_index(&serde_json::to_vec(&index).unwrap()).unwrap();

        assert_eq!(catalog.plugins.len(), 1);
        assert_eq!(catalog.plugins[0].version, "1.0.0");
        assert_eq!(catalog.problems.len(), 7, "{:?}", catalog.problems);
        for fragment in [
            "https address",
            "SHA-256",
            "4 MiB",
            "version",
            "not valid",
            "listed twice",
            "Entry 8",
        ] {
            assert!(
                catalog.problems.iter().any(|p| p.contains(fragment)),
                "{fragment}: {:?}",
                catalog.problems
            );
        }

        // A newer format is not guessed at.
        index["version"] = serde_json::json!(2);
        let error = parse_index(&serde_json::to_vec(&index).unwrap()).unwrap_err();
        assert!(error.contains("format 2"), "{error}");
        assert!(parse_index(b"{ nope").is_err());
        assert!(parse_index(b"[]").is_err());
    }

    #[test]
    fn only_so_many_plugins_are_listed() {
        let entries: Vec<_> = (0..MAX_CATALOG_PLUGINS + 5)
            .map(|n| entry_for(&minimal(), &format!("p{n}"), "1.0.0"))
            .collect();
        let catalog = parse_index(&index_with(&entries)).unwrap();

        assert_eq!(catalog.plugins.len(), MAX_CATALOG_PLUGINS);
        assert!(catalog.problems[0].contains("first 128"));
    }

    // --- downloading a package -----------------------------------------------------------------

    fn packages(bytes: &[u8], entry: &CatalogEntry) -> Fake {
        Fake(HashMap::from([(entry.url.clone(), bytes.to_vec())]))
    }

    #[test]
    fn a_package_that_is_what_the_index_promised_is_accepted() {
        let bytes = minimal();
        let entry = entry_for(&bytes, "hello", "1.0.0");

        assert_eq!(download(&packages(&bytes, &entry), &entry).unwrap(), bytes);
    }

    #[test]
    fn a_package_that_is_not_what_the_index_promised_is_refused() {
        let bytes = minimal();
        let entry = entry_for(&bytes, "hello", "1.0.0");

        // A different file at the same address: wrong checksum (same size).
        let mut swapped = bytes.clone();
        let last = swapped.len() - 1;
        swapped[last] ^= 1;
        let error = download(&packages(&swapped, &entry), &entry).unwrap_err();
        assert!(error.contains("checksum"), "{error}");

        // The wrong size.
        let mut short = entry.clone();
        short.size += 1;
        short.sha256 = entry.sha256.clone();
        assert!(
            download(&packages(&bytes, &short), &short)
                .unwrap_err()
                .contains("size")
        );

        // Listed as another plugin, another version, or with other permissions than the file asks for.
        let wrong_id = entry_for(&bytes, "other", "1.0.0");
        assert!(
            download(&packages(&bytes, &wrong_id), &wrong_id)
                .unwrap_err()
                .contains("not what the catalog listed")
        );
        let wrong_version = entry_for(&bytes, "hello", "9.9.9");
        assert!(
            download(&packages(&bytes, &wrong_version), &wrong_version)
                .unwrap_err()
                .contains("not what the catalog listed")
        );
        let mut wrong_permissions = entry.clone();
        wrong_permissions.permissions = vec![Permission::Blocks, Permission::Notify];
        assert!(
            download(&packages(&bytes, &wrong_permissions), &wrong_permissions)
                .unwrap_err()
                .contains("other permissions")
        );

        // Something that is not a plugin, even with a matching checksum.
        let junk = b"\0asm junk".to_vec();
        let junk_entry = entry_for(&junk, "hello", "1.0.0");
        assert!(download(&packages(&junk, &junk_entry), &junk_entry).is_err());

        // A server that answers 404, and an entry that is not valid to begin with.
        assert!(
            download(&Fake(HashMap::new()), &entry)
                .unwrap_err()
                .contains("Could not download")
        );
        let mut http = entry.clone();
        http.url = "http://x.test/p.wasm".to_owned();
        assert!(
            download(&packages(&bytes, &http), &http)
                .unwrap_err()
                .contains("cannot be used")
        );
        // Permissions in a different order are the same permissions.
        let multi = MINIMAL.replace("[\"blocks\"]", "[\"blocks\", \"notify\"]");
        let file = plugin(&multi, "");
        let mut entry = entry_for(&file, "hello", "1.0.0");
        entry.permissions = vec![Permission::Notify, Permission::Blocks];
        assert!(download(&packages(&file, &entry), &entry).is_ok());
    }

    #[test]
    fn an_update_is_a_higher_version() {
        assert!(is_newer("1.0.0", "1.0.1"));
        assert!(is_newer("1.9.0", "1.10.0"));
        assert!(is_newer("1.0.0-beta", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("junk", "1.0.0"));
    }

    // --- the real HTTP client, against a server on this machine --------------------------------

    /// Serves one request on a free port with `respond`, which gets the stream after the request was read.
    fn serve(respond: impl FnOnce(&mut std::net::TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer);
                respond(&mut stream);
            }
        });
        format!("http://{address}/file")
    }

    fn reply(stream: &mut std::net::TcpStream, status: &str, body: &[u8]) {
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body);
    }

    #[test]
    fn the_client_fetches_within_its_limits() {
        let client = HttpFetch::insecure_for_tests(Duration::from_secs(5));

        let url = serve(|stream| reply(stream, "200 OK", b"hello"));
        assert_eq!(client.get(&url, 1024).unwrap(), b"hello");

        let url = serve(|stream| reply(stream, "404 Not Found", b"nope"));
        assert!(client.get(&url, 1024).unwrap_err().contains("404"));

        let url = serve(|stream| reply(stream, "200 OK", &vec![b'x'; 5000]));
        assert!(client.get(&url, 1024).unwrap_err().contains("larger"));
    }

    #[test]
    fn an_answer_of_exactly_the_limit_is_accepted() {
        // `download` asks for as many bytes as the signed index promises, so the limit is often hit exactly.
        let client = HttpFetch::insecure_for_tests(Duration::from_secs(5));
        let body = vec![b'x'; 341];

        let url = serve({
            let body = body.clone();
            move |stream| reply(stream, "200 OK", &body)
        });
        assert_eq!(client.get(&url, 341).unwrap(), body);

        // One byte more is still too much.
        let url = serve(move |stream| reply(stream, "200 OK", &vec![b'x'; 342]));
        assert!(client.get(&url, 341).unwrap_err().contains("larger"));
    }

    #[test]
    fn the_client_gives_up_on_a_server_that_does_not_answer() {
        let client = HttpFetch::insecure_for_tests(Duration::from_millis(500));
        let url = serve(|_stream| std::thread::sleep(Duration::from_secs(3)));

        let started = std::time::Instant::now();
        let error = client.get(&url, 1024).unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "{:?}",
            started.elapsed()
        );
        assert!(
            error.contains("too long") || error.contains("Could not"),
            "{error}"
        );
    }

    #[test]
    fn the_real_client_never_speaks_plain_http() {
        let url = serve(|stream| reply(stream, "200 OK", b"secret"));
        let error = HttpFetch::new().get(&url, 1024).unwrap_err();
        assert!(!error.is_empty());
        // And a redirect from https to http is refused the same way (https_only covers every hop).
        let url = serve(|stream| {
            let _ = stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/x\r\nContent-Length: 0\r\n\r\n");
        });
        assert!(HttpFetch::new().get(&url, 1024).is_err());
    }
}
