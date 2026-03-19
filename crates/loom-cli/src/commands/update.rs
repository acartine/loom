use flate2::read::GzDecoder;
use miette::{Context, IntoDiagnostic};
use reqwest::blocking::{Client, Response};
use reqwest::header::LOCATION;
use semver::Version;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use tar::Archive;
use tempfile::{tempdir, NamedTempFile};

const DEFAULT_BASE_URL: &str = "https://github.com/acartine/loom";
const CHECKSUM_FILE: &str = "loom-checksums.txt";
const BIN_NAME: &str = "loom";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseTarget {
    archive_name: String,
    triple: String,
}

impl ReleaseTarget {
    fn new(triple: &str) -> Self {
        Self {
            archive_name: format!("loom-{triple}.tar.gz"),
            triple: triple.to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct ReleaseUrls {
    archive_url: String,
    checksums_url: String,
}

pub fn run(check: bool, force: bool) -> miette::Result<()> {
    let target = detect_release_target()?;
    let executable = current_executable_path()?;
    validate_install_location(&executable)?;

    let client = build_client()?;
    let urls = release_urls(&release_base_url(), &target, None);
    let latest_tag = resolve_latest_tag(&client, &urls.archive_url)?;
    let latest_version = normalize_version(&latest_tag)?;
    let current_version = normalize_version(VERSION)?;

    if check {
        if latest_version > current_version {
            println!("Update available: {VERSION} -> {latest_tag}");
        } else if latest_version == current_version {
            println!("Already up to date: {VERSION}");
        } else {
            println!("Current version {VERSION} is newer than latest release {latest_tag}");
        }
        return Ok(());
    }

    if latest_version <= current_version && !force {
        println!("Already up to date: {VERSION}");
        return Ok(());
    }

    ensure_parent_writable(&executable)?;

    let tmpdir = tempdir().into_diagnostic()?;
    let archive_path = tmpdir.path().join(&target.archive_name);
    let checksums_path = tmpdir.path().join(CHECKSUM_FILE);

    let resolved_archive_url = download_to_path(&client, &urls.archive_url, &archive_path)
        .wrap_err("failed to download release archive")?;
    let resolved_tag = parse_release_tag_from_url(resolved_archive_url.as_str())
        .wrap_err("failed to infer resolved release version from redirected archive URL")?;
    let checksum_urls = release_urls(&release_base_url(), &target, Some(&resolved_tag));
    download_to_path(&client, &checksum_urls.checksums_url, &checksums_path)
        .wrap_err("failed to download release checksums")?;

    verify_checksum(&archive_path, &target.archive_name, &checksums_path)?;

    let extracted_binary = extract_binary(&archive_path, tmpdir.path())?;
    install_binary(&extracted_binary, &executable)?;

    println!(
        "Updated {BIN_NAME} to {resolved_tag} at {}",
        executable.display()
    );
    Ok(())
}

fn build_client() -> miette::Result<Client> {
    Client::builder()
        .user_agent(format!("loom-cli/{VERSION}"))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .into_diagnostic()
}

fn release_base_url() -> String {
    env::var("LOOM_UPDATE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned())
}

fn detect_release_target() -> miette::Result<ReleaseTarget> {
    let os = env::var("LOOM_UPDATE_TEST_OS").unwrap_or_else(|_| env::consts::OS.to_owned());
    let arch = env::var("LOOM_UPDATE_TEST_ARCH").unwrap_or_else(|_| env::consts::ARCH.to_owned());
    map_target(&os, &arch)
}

fn map_target(os: &str, arch: &str) -> miette::Result<ReleaseTarget> {
    match (os, arch) {
        ("linux", "x86_64") | ("linux", "amd64") => {
            Ok(ReleaseTarget::new("x86_64-unknown-linux-musl"))
        }
        ("linux", "aarch64") | ("linux", "arm64") => {
            Ok(ReleaseTarget::new("aarch64-unknown-linux-musl"))
        }
        ("macos", "aarch64") | ("macos", "arm64") => {
            Ok(ReleaseTarget::new("aarch64-apple-darwin"))
        }
        ("macos", "x86_64") | ("macos", "amd64") => Err(miette::miette!(
            "unsupported platform: macOS x86_64 has no published Loom release artifact"
        )),
        _ => Err(miette::miette!(
            "unsupported platform: {os} {arch}. Supported targets: x86_64-unknown-linux-musl, aarch64-unknown-linux-musl, aarch64-apple-darwin"
        )),
    }
}

fn release_urls(base_url: &str, target: &ReleaseTarget, tag: Option<&str>) -> ReleaseUrls {
    let trimmed = base_url.trim_end_matches('/');
    let archive_url = match tag {
        Some(tag) => format!("{trimmed}/releases/download/{tag}/{}", target.archive_name),
        None => format!("{trimmed}/releases/latest/download/{}", target.archive_name),
    };
    let checksums_url = match tag {
        Some(tag) => format!("{trimmed}/releases/download/{tag}/{CHECKSUM_FILE}"),
        None => format!("{trimmed}/releases/latest/download/{CHECKSUM_FILE}"),
    };

    ReleaseUrls {
        archive_url,
        checksums_url,
    }
}

fn resolve_latest_tag(client: &Client, archive_url: &str) -> miette::Result<String> {
    let response = client.head(archive_url).send().into_diagnostic()?;
    let final_url = final_url(&response)
        .or_else(|| redirected_location(response.headers().get(LOCATION), archive_url))
        .ok_or_else(|| miette::miette!("missing final URL while resolving latest release"))?;
    parse_release_tag_from_url(&final_url)
}

fn final_url(response: &Response) -> Option<String> {
    let url = response.url().as_str();
    if url.is_empty() {
        None
    } else {
        Some(url.to_owned())
    }
}

fn redirected_location(
    location: Option<&reqwest::header::HeaderValue>,
    base: &str,
) -> Option<String> {
    let location = location?.to_str().ok()?;
    if location.starts_with("http://") || location.starts_with("https://") {
        Some(location.to_owned())
    } else {
        Some(format!("{}{}", base.trim_end_matches('/'), location))
    }
}

fn parse_release_tag_from_url(url: &str) -> miette::Result<String> {
    let parts: Vec<_> = url.split('/').collect();
    let download_index = parts
        .iter()
        .position(|part| *part == "download")
        .ok_or_else(|| {
            miette::miette!("redirected URL does not include a release download path")
        })?;
    let tag = parts
        .get(download_index + 1)
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| miette::miette!("redirected URL is missing the release tag"))?;
    Ok((*tag).to_owned())
}

fn normalize_version(raw: &str) -> miette::Result<Version> {
    Version::parse(raw.trim_start_matches('v')).into_diagnostic()
}

fn current_executable_path() -> miette::Result<PathBuf> {
    env::current_exe().into_diagnostic()
}

fn validate_install_location(path: &Path) -> miette::Result<()> {
    if path.file_name() != Some(OsStr::new(BIN_NAME)) {
        return Err(miette::miette!(
            "refusing to self-update from {}: the executable does not look like an installed `{BIN_NAME}` binary",
            path.display()
        ));
    }

    if !looks_like_installed_binary(path) {
        return Err(miette::miette!(
            "refusing to self-update from {}: install Loom into ~/.local/bin, /usr/local/bin, /usr/bin, or /opt/homebrew/bin first",
            path.display()
        ));
    }

    Ok(())
}

fn looks_like_installed_binary(path: &Path) -> bool {
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(part) if part == ".local"))
        && path.parent().and_then(Path::file_name) == Some(OsStr::new("bin"))
    {
        return true;
    }

    const PREFIXES: &[&str] = &["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"];
    PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

fn ensure_parent_writable(path: &Path) -> miette::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        miette::miette!(
            "cannot update {} because it has no parent directory",
            path.display()
        )
    })?;
    let probe = NamedTempFile::new_in(parent)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot write to {}", parent.display()))?;
    drop(probe);
    Ok(())
}

fn download_to_path(client: &Client, url: &str, destination: &Path) -> miette::Result<String> {
    let mut response = client.get(url).send().into_diagnostic()?;
    if !response.status().is_success() {
        return Err(miette::miette!(
            "download failed for {url}: HTTP {}",
            response.status()
        ));
    }

    let final_url = response.url().to_string();
    let mut file = File::create(destination).into_diagnostic()?;
    io::copy(&mut response, &mut file).into_diagnostic()?;
    Ok(final_url)
}

fn verify_checksum(
    archive_path: &Path,
    archive_name: &str,
    checksums_path: &Path,
) -> miette::Result<()> {
    let expected = checksum_for(archive_name, checksums_path)?;
    let mut file = File::open(archive_path).into_diagnostic()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer).into_diagnostic()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let actual = format!("{:x}", hasher.finalize());
    if actual != expected {
        return Err(miette::miette!(
            "checksum verification failed for {archive_name}: expected {expected}, got {actual}"
        ));
    }

    Ok(())
}

fn checksum_for(archive_name: &str, checksums_path: &Path) -> miette::Result<String> {
    let file = File::open(checksums_path).into_diagnostic()?;
    for line in BufReader::new(file).lines() {
        let line = line.into_diagnostic()?;
        let mut parts = line.split_whitespace();
        let checksum = parts.next();
        let filename = parts.next();
        if filename == Some(archive_name) {
            return Ok(checksum.unwrap_or_default().to_owned());
        }
    }

    Err(miette::miette!(
        "checksum file {} does not contain an entry for {archive_name}",
        checksums_path.display()
    ))
}

fn extract_binary(archive_path: &Path, destination_dir: &Path) -> miette::Result<PathBuf> {
    let file = File::open(archive_path).into_diagnostic()?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(destination_dir).into_diagnostic()?;
    let binary_path = destination_dir.join(BIN_NAME);
    if !binary_path.exists() {
        return Err(miette::miette!(
            "release archive {} does not contain `{BIN_NAME}`",
            archive_path.display()
        ));
    }
    Ok(binary_path)
}

fn install_binary(source: &Path, destination: &Path) -> miette::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        miette::miette!(
            "cannot update {} because it has no parent directory",
            destination.display()
        )
    })?;
    let mut staged = NamedTempFile::new_in(parent).into_diagnostic()?;
    let mut source_file = File::open(source).into_diagnostic()?;
    io::copy(&mut source_file, staged.as_file_mut()).into_diagnostic()?;
    staged
        .as_file_mut()
        .set_permissions(fs::Permissions::from_mode(0o755))
        .into_diagnostic()?;
    staged.as_file_mut().flush().into_diagnostic()?;
    staged.persist(destination).into_diagnostic()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_supported_targets() {
        assert_eq!(
            map_target("linux", "x86_64").unwrap(),
            ReleaseTarget::new("x86_64-unknown-linux-musl")
        );
        assert_eq!(
            map_target("linux", "aarch64").unwrap(),
            ReleaseTarget::new("aarch64-unknown-linux-musl")
        );
        assert_eq!(
            map_target("macos", "arm64").unwrap(),
            ReleaseTarget::new("aarch64-apple-darwin")
        );
    }

    #[test]
    fn rejects_unsupported_targets() {
        let err = map_target("macos", "x86_64").unwrap_err().to_string();
        assert!(err.contains("macOS x86_64"));

        let err = map_target("freebsd", "x86_64").unwrap_err().to_string();
        assert!(err.contains("unsupported platform"));
    }

    #[test]
    fn builds_release_urls() {
        let target = ReleaseTarget::new("aarch64-apple-darwin");
        let latest = release_urls("https://example.com/acartine/loom", &target, None);
        assert_eq!(
            latest.archive_url,
            "https://example.com/acartine/loom/releases/latest/download/loom-aarch64-apple-darwin.tar.gz"
        );
        assert_eq!(
            latest.checksums_url,
            "https://example.com/acartine/loom/releases/latest/download/loom-checksums.txt"
        );

        let tagged = release_urls("https://example.com/acartine/loom", &target, Some("v1.2.3"));
        assert_eq!(
            tagged.archive_url,
            "https://example.com/acartine/loom/releases/download/v1.2.3/loom-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn parses_redirected_version() {
        let tag = parse_release_tag_from_url(
            "https://github.com/acartine/loom/releases/download/v1.2.3/loom-x86_64-unknown-linux-musl.tar.gz",
        )
        .unwrap();
        assert_eq!(tag, "v1.2.3");
    }

    #[test]
    fn recognizes_installed_paths() {
        assert!(looks_like_installed_binary(Path::new(
            "/tmp/test/.local/bin/loom"
        )));
        assert!(looks_like_installed_binary(Path::new(
            "/usr/local/bin/loom"
        )));
        assert!(!looks_like_installed_binary(Path::new(
            "/tmp/project/target/debug/loom"
        )));
    }
}
