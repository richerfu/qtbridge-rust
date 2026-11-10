// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use qtbridge_build_utils::qt_build::QtInstallation;

// Run discovery in a child process so tests never mutate the test runner's
// environment. The fake tools distinguish qmake target metadata from qtpaths.
#[test]
#[ignore = "invoked by the Qt discovery regression tests"]
fn discovery_probe() {
    let qt = QtInstallation::default();
    println!("qtbridge-includes={:?}", qt.include_dirs(["Core"], true));
    qt.link_modules(["Core"]);
}

struct QtFixture {
    root: PathBuf,
    bin: PathBuf,
}

impl QtFixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "qtbridge qt discovery {}-{nonce}",
            std::process::id(),
        ));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        for installation in ["desktop", "ohos"] {
            fs::create_dir_all(root.join(installation).join("lib/QtCore.framework")).unwrap();
        }

        let fixture = Self { root, bin };
        let target_headers = fixture.root.join("ohos/include");
        let target_libs = fixture.root.join("ohos/lib");
        let properties = format!(
            "QT_VERSION:6.12.0\nQT_INSTALL_HEADERS:{}\nQT_INSTALL_LIBS:{}\n",
            target_headers.display(),
            target_libs.display(),
        );
        let bin = fixture.bin.display();
        fixture.write_executable(
            "qmake",
            &format!(
                "#!/bin/sh\n\
             if [ \"$1\" = '-query' ]; then\n\
               case \"$2\" in\n\
                 '') printf '%s' '{properties}' ;;\n\
                 QT_VERSION) printf '%s\\n' '6.12.0' ;;\n\
                 QT_HOST_LIBEXECS/get) printf '%s\\n' '{bin}' ;;\n\
                 *) exit 1 ;;\n\
               esac\n\
             fi\n",
            ),
        );
        let desktop_headers = fixture.root.join("desktop/include");
        let desktop_libs = fixture.root.join("desktop/lib");
        fixture.write_executable(
            "qtpaths",
            &format!(
                "#!/bin/sh\n\
             if [ \"$1\" = '--query' ]; then\n\
               case \"$2\" in\n\
                 QT_VERSION) printf '%s\\n' '6.12.0' ;;\n\
                 QT_INSTALL_HEADERS) printf '%s\\n' '{}' ;;\n\
                 QT_INSTALL_LIBS) printf '%s\\n' '{}' ;;\n\
                 *) exit 1 ;;\n\
               esac\n\
             fi\n",
                desktop_headers.display(),
                desktop_libs.display(),
            ),
        );
        fixture
    }

    fn write_executable(&self, name: &str, content: &str) {
        let path = self.bin.join(name);
        fs::write(&path, content).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn probe(&self, target_env: &str, cross: bool, explicit_qmake: bool) -> Output {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", "discovery_probe", "--nocapture"])
            .env("PATH", &self.bin)
            .env("HOST", "native-host")
            .env("TARGET", if cross { "cross-target" } else { "native-host" })
            .env("CARGO_CFG_TARGET_ENV", target_env)
            .env(
                "CARGO_CFG_TARGET_OS",
                if target_env == "ohos" {
                    "linux"
                } else {
                    std::env::consts::OS
                },
            )
            .env("QT_VERSION_MAJOR", "6")
            .env_remove("QMAKE");
        if explicit_qmake {
            command.env("QMAKE", self.bin.join("qmake"));
        }
        command.output().unwrap()
    }

    fn assert_desktop(&self, output: Output) {
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(output.status.success(), "{stdout}\n{stderr}");
        assert!(
            stdout.contains(self.root.join("desktop/include").to_str().unwrap()),
            "{stdout}"
        );
        assert!(
            !stdout.contains(self.root.join("ohos/include").to_str().unwrap()),
            "{stdout}"
        );
        let expected_link = if cfg!(target_os = "macos") {
            "cargo::rustc-link-lib=framework=QtCore"
        } else {
            "cargo::rustc-link-lib=Qt6Core"
        };
        assert!(stdout.contains(expected_link), "{stdout}");
    }
}

impl Drop for QtFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn desktop_native_build_keeps_qtpaths_discovery() {
    let fixture = QtFixture::new();
    fixture.assert_desktop(fixture.probe("", false, false));
}

#[test]
fn desktop_explicit_qmake_keeps_qtpaths_metadata() {
    let fixture = QtFixture::new();
    fixture.assert_desktop(fixture.probe("", false, true));
}

#[test]
fn non_ohos_cross_build_does_not_require_qmake() {
    let fixture = QtFixture::new();
    fixture.assert_desktop(fixture.probe("gnu", true, false));
}

#[test]
fn ohos_uses_target_metadata_without_host_frameworks() {
    let fixture = QtFixture::new();
    let output = fixture.probe("ohos", true, true);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains(fixture.root.join("ohos/include").to_str().unwrap()),
        "{stdout}"
    );
    assert!(
        !stdout.contains(fixture.root.join("desktop/include").to_str().unwrap()),
        "{stdout}"
    );
    assert!(stdout.contains("cargo::rustc-link-lib=Qt6Core"), "{stdout}");
    assert!(
        !stdout.contains("cargo::rustc-link-lib=framework="),
        "{stdout}"
    );
}

#[test]
fn ohos_requires_explicit_target_qmake() {
    let fixture = QtFixture::new();
    let output = fixture.probe("ohos", true, false);
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Building QtBridge for OHOS requires QMAKE"));
}
