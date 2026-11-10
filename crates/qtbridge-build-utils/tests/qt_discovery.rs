// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use qtbridge_build_utils::qt_build::QtInstallation;

// Isolate Cargo/Qt environment variables in a child process.
#[test]
#[ignore = "invoked by the Qt discovery regression tests"]
fn discovery_probe() {
    let qt = QtInstallation::default();
    println!("qtbridge-includes={:?}", qt.include_dirs(["Core"], true));
    qt.link_modules(["Core"]);
}

struct QtFixture {
    root: PathBuf,
}

impl QtFixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fixture = Self {
            root: std::env::temp_dir().join(format!(
                "qtbridge qt discovery {}-{nonce}",
                std::process::id()
            )),
        };
        fs::create_dir_all(fixture.root.join("bin")).unwrap();
        for (tool, installation) in [("qmake", "target"), ("qtpaths", "desktop")] {
            fs::create_dir_all(fixture.root.join(installation).join("lib/QtCore.framework"))
                .unwrap();
            fixture.write_executable(tool, &format!(r#"#!/bin/sh
case "$1:$2" in
  -query:|--query:) printf '%s\n' 'QT_VERSION:6.12.0' 'QT_INSTALL_HEADERS:{root}/{installation}/include' 'QT_INSTALL_LIBS:{root}/{installation}/lib' ;;
  *:QT_VERSION) printf '%s\n' '6.12.0' ;;
  *:QT_HOST_LIBEXECS/get) printf '%s\n' '{root}/bin' ;;
  *:QT_INSTALL_HEADERS) printf '%s\n' '{root}/{installation}/include' ;;
  *:QT_INSTALL_LIBS) printf '%s\n' '{root}/{installation}/lib' ;;
  -help:|--help:) ;;
  *) exit 1 ;;
esac
"#, root = fixture.root.display()));
        }
        fixture
    }

    fn write_executable(&self, name: &str, content: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, content).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn probe(&self, target_os: &str, cross: bool, explicit_qmake: bool) -> Output {
        let bin = self.root.join("bin");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", "discovery_probe", "--nocapture"])
            .env("PATH", &bin)
            .env("HOST", "native-host")
            .env("TARGET", if cross { "cross-target" } else { "native-host" })
            .env("CARGO_CFG_TARGET_OS", target_os)
            .env("QT_VERSION_MAJOR", "6")
            .env_remove("QMAKE");
        if explicit_qmake {
            command.env("QMAKE", bin.join("qmake"));
        }
        command.output().unwrap()
    }

    fn assert_discovery(&self, output: Output, installation: &str, frameworks: bool) {
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "{stdout}\n{stderr}");
        for source in ["desktop", "target"] {
            let headers = self.root.join(source).join("include");
            assert_eq!(
                stdout.contains(headers.to_str().unwrap()),
                source == installation,
                "{stdout}"
            );
        }
        let library = if frameworks {
            "framework=QtCore"
        } else {
            "Qt6Core"
        };
        assert!(
            stdout.contains(&format!("cargo::rustc-link-lib={library}")),
            "{stdout}"
        );
        assert_eq!(
            stdout.contains("QtCore.framework/Headers"),
            frameworks,
            "{stdout}"
        );
    }
}

impl Drop for QtFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn native_build_keeps_qtpaths_discovery() {
    let fixture = QtFixture::new();
    for explicit_qmake in [false, true] {
        let output = fixture.probe(std::env::consts::OS, false, explicit_qmake);
        fixture.assert_discovery(output, "desktop", cfg!(target_os = "macos"));
    }
}

#[test]
fn cross_build_uses_target_metadata_and_frameworks() {
    let fixture = QtFixture::new();
    for (target_os, explicit_qmake, frameworks) in [
        ("linux", false, false),
        ("linux", true, false),
        ("windows", true, false),
        ("macos", true, true),
        ("ios", true, true),
    ] {
        let output = fixture.probe(target_os, true, explicit_qmake);
        fixture.assert_discovery(output, "target", frameworks);
    }
}

#[test]
fn cross_build_to_macos_with_plain_libraries() {
    let fixture = QtFixture::new();
    fs::remove_dir(fixture.root.join("target/lib/QtCore.framework")).unwrap();
    fixture.assert_discovery(fixture.probe("macos", true, true), "target", false);
}

#[test]
fn incomplete_target_metadata_is_rejected() {
    let fixture = QtFixture::new();
    fixture.write_executable("qmake", "#!/bin/sh\nif [ -z \"$2\" ]; then printf 'QT_VERSION:6.12.0\\n'; else printf '6.12.0\\n'; fi\n");
    for explicit_qmake in [false, true] {
        let output = fixture.probe("linux", true, explicit_qmake);
        assert!(!output.status.success());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("-query is missing QT_INSTALL_HEADERS"));
    }
}
