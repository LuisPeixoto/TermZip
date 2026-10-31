//! Writes the catalog the maintainers sign: copies each plugin to `<out>/<id>.wasm` and lists them in
//! `<out>/index.json`.
//!
//! Everything in an entry comes from the plugin file, read with the reader the app uses, so nothing is typed in
//! by hand. The index is then read back with the app's own parser before it is written: a catalog that gets
//! signed is a catalog TermZip can read, with nothing left out.
//!
//! ```text
//! cargo run -p termzip-plugins --example catalog_index -- \
//!     --plugins DIR --out DIR --base-url https://github.com/OWNER/REPO/releases/download/TAG
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use termzip_plugins::{CatalogEntry, INDEX_FORMAT, parse_index, read_module, sha256_hex};

struct Arguments {
    plugins: PathBuf,
    out: PathBuf,
    base_url: String,
}

fn arguments() -> Result<Arguments, String> {
    let (mut plugins, mut out, mut base_url) = (None, None, None);
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--plugins" => plugins = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            "--base-url" => base_url = Some(value),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(Arguments {
        plugins: plugins.ok_or("--plugins is required")?,
        out: out.ok_or("--out is required")?,
        base_url: base_url.ok_or("--base-url is required")?,
    })
}

fn entries(plugins: &Path, base_url: &str) -> Result<Vec<(CatalogEntry, Vec<u8>)>, String> {
    let mut files: Vec<PathBuf> = fs::read_dir(plugins)
        .map_err(|error| format!("cannot read {}: {error}", plugins.display()))?
        .filter_map(Result::ok)
        .map(|file| file.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wasm")
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("there are no .wasm files in {}", plugins.display()));
    }

    let mut found: Vec<(CatalogEntry, Vec<u8>)> = Vec::new();
    for path in files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let bytes = fs::read(&path).map_err(|error| format!("{name}: {error}"))?;
        let manifest = read_module(&bytes)
            .map_err(|error| format!("{name}: {error}"))?
            .manifest;
        if found.iter().any(|(entry, _)| entry.id == manifest.id) {
            return Err(format!(
                "{name}: the id “{}” is already listed",
                manifest.id
            ));
        }
        let entry = CatalogEntry {
            url: format!("{}/{}.wasm", base_url.trim_end_matches('/'), manifest.id),
            sha256: sha256_hex(&bytes),
            size: bytes.len() as u64,
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            author: manifest.author,
            license: manifest.license,
            homepage: manifest.homepage,
            permissions: manifest.permissions,
        };
        found.push((entry, bytes));
    }
    found.sort_by(|(a, _), (b, _)| a.id.cmp(&b.id));
    Ok(found)
}

fn run() -> Result<Vec<String>, String> {
    let Arguments {
        plugins,
        out,
        base_url,
    } = arguments()?;
    let found = entries(&plugins, &base_url)?;

    let index = serde_json::json!({
        "version": INDEX_FORMAT,
        "plugins": found.iter().map(|(entry, _)| entry).collect::<Vec<_>>(),
    });
    let mut text = serde_json::to_string_pretty(&index).map_err(|error| error.to_string())?;
    text.push('\n');

    // What is signed must be what the app reads: every entry, none left out.
    let read = parse_index(text.as_bytes())?;
    if !read.problems.is_empty() {
        return Err(read.problems.join("\n"));
    }
    if read.plugins.len() != found.len() {
        return Err("the app would not list every plugin".to_owned());
    }

    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    for (entry, bytes) in &found {
        fs::write(out.join(format!("{}.wasm", entry.id)), bytes)
            .map_err(|error| format!("{}: {error}", entry.id))?;
    }
    fs::write(out.join("index.json"), text).map_err(|error| format!("index.json: {error}"))?;

    Ok(found
        .iter()
        .map(|(entry, _)| {
            format!(
                "{} {} {} bytes {}",
                entry.id,
                entry.version,
                entry.size,
                &entry.sha256[..12]
            )
        })
        .collect())
}

fn main() -> ExitCode {
    match run() {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
