mod common;

use common::TestInstall;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

struct StubReleaseServer {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl StubReleaseServer {
    fn start(
        tag: &str,
        archive_name: &str,
        archive_bytes: Vec<u8>,
        checksums_bytes: Vec<u8>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub server");
        listener
            .set_nonblocking(true)
            .expect("set listener nonblocking");
        let addr = listener.local_addr().expect("local addr");
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let tag = tag.to_owned();
        let archive_name = archive_name.to_owned();

        let handle = thread::spawn(move || {
            while !shutdown_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        handle_connection(
                            stream,
                            &tag,
                            &archive_name,
                            &archive_bytes,
                            &checksums_bytes,
                        );
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(err) => panic!("stub server accept failed: {err}"),
                }
            }
        });

        Self {
            addr,
            shutdown,
            handle: Some(handle),
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}/acartine/loom", self.addr)
    }
}

impl Drop for StubReleaseServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("join stub server");
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    tag: &str,
    archive_name: &str,
    archive_bytes: &[u8],
    checksums_bytes: &[u8],
) {
    stream.set_nonblocking(false).expect("set stream blocking");
    let mut request = [0_u8; 4096];
    let read = stream.read(&mut request).expect("read request");
    let request_line = String::from_utf8_lossy(&request[..read]);
    let first_line = request_line.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    let (status, headers, body) = match path {
        p if p == format!("/acartine/loom/releases/latest/download/{archive_name}") => (
            "302 Found",
            vec![(
                "Location",
                format!("/acartine/loom/releases/download/{tag}/{archive_name}"),
            )],
            Vec::new(),
        ),
        "/acartine/loom/releases/latest/download/loom-checksums.txt" => (
            "302 Found",
            vec![(
                "Location",
                format!("/acartine/loom/releases/download/{tag}/loom-checksums.txt"),
            )],
            Vec::new(),
        ),
        p if p == format!("/acartine/loom/releases/download/{tag}/{archive_name}") => (
            "200 OK",
            vec![("Content-Type", "application/gzip".to_owned())],
            if method == "HEAD" {
                Vec::new()
            } else {
                archive_bytes.to_vec()
            },
        ),
        p if p == format!("/acartine/loom/releases/download/{tag}/loom-checksums.txt") => (
            "200 OK",
            vec![("Content-Type", "text/plain".to_owned())],
            if method == "HEAD" {
                Vec::new()
            } else {
                checksums_bytes.to_vec()
            },
        ),
        _ => ("404 Not Found", vec![], b"not found".to_vec()),
    };

    let mut response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n", body.len());
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("Connection: close\r\n\r\n");
    stream
        .write_all(response.as_bytes())
        .expect("write response headers");
    if method != "HEAD" {
        stream.write_all(&body).expect("write response body");
    }
}

fn make_release_archive(binary_contents: &[u8]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let encoder = GzEncoder::new(&mut compressed, Compression::default());
    let mut builder = tar::Builder::new(encoder);

    let mut header = tar::Header::new_gnu();
    header.set_mode(0o755);
    header.set_size(binary_contents.len() as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, "loom", binary_contents)
        .expect("append binary to archive");
    let encoder = builder.into_inner().expect("finish tar builder");
    encoder.finish().expect("finish gzip encoder");
    compressed
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn command_env<'a>(command: &'a mut Command, base_url: &str) -> &'a mut Command {
    command
        .env("LOOM_UPDATE_BASE_URL", base_url)
        .env("LOOM_UPDATE_TEST_OS", "linux")
        .env("LOOM_UPDATE_TEST_ARCH", "x86_64")
}

#[test]
fn update_check_reports_available_version() {
    let install = TestInstall::new();
    let archive_name = "loom-x86_64-unknown-linux-musl.tar.gz";
    let archive_bytes = make_release_archive(b"#!/bin/sh\necho updated\n");
    let checksums = format!("{}  {}\n", sha256_hex(&archive_bytes), archive_name);
    let server = StubReleaseServer::start(
        "v9.9.9",
        archive_name,
        archive_bytes,
        checksums.into_bytes(),
    );

    let output = command_env(&mut Command::new(&install.executable), &server.base_url())
        .args(["update", "--check"])
        .output()
        .expect("run loom update --check");

    assert!(
        output.status.success(),
        "update --check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Update available"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn update_noops_when_already_current() {
    let install = TestInstall::new();
    let original = fs::read(&install.executable).expect("read original binary");
    let archive_name = "loom-x86_64-unknown-linux-musl.tar.gz";
    let archive_bytes = make_release_archive(&original);
    let checksums = format!("{}  {}\n", sha256_hex(&archive_bytes), archive_name);
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let server =
        StubReleaseServer::start(&tag, archive_name, archive_bytes, checksums.into_bytes());

    let output = command_env(&mut Command::new(&install.executable), &server.base_url())
        .arg("update")
        .output()
        .expect("run loom update");

    assert!(
        output.status.success(),
        "update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Already up to date"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        fs::read(&install.executable).expect("read installed binary after update"),
        original
    );
}

#[test]
fn update_force_reinstalls_current_version() {
    let install = TestInstall::new();
    let archive_name = "loom-x86_64-unknown-linux-musl.tar.gz";
    let replacement = b"#!/bin/sh\necho forced-update\n";
    let archive_bytes = make_release_archive(replacement);
    let checksums = format!("{}  {}\n", sha256_hex(&archive_bytes), archive_name);
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let server =
        StubReleaseServer::start(&tag, archive_name, archive_bytes, checksums.into_bytes());

    let output = command_env(&mut Command::new(&install.executable), &server.base_url())
        .args(["update", "--force"])
        .output()
        .expect("run loom update --force");

    assert!(
        output.status.success(),
        "update --force failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let updated = fs::read(&install.executable).expect("read updated binary");
    assert_eq!(updated, replacement);
}

#[test]
fn update_rejects_unsupported_targets() {
    let install = TestInstall::new();
    let output = Command::new(&install.executable)
        .args(["update", "--check"])
        .env("LOOM_UPDATE_TEST_OS", "macos")
        .env("LOOM_UPDATE_TEST_ARCH", "x86_64")
        .output()
        .expect("run loom update --check on unsupported target");

    assert!(!output.status.success(), "command unexpectedly succeeded");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("macOS x86_64"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
