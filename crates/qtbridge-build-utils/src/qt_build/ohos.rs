// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub(super) struct OhosQtInstallation {
    properties: BTreeMap<String, String>,
}

impl OhosQtInstallation {
    pub(super) fn discover() -> Option<Self> {
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("ohos") {
            return None;
        }
        println!("cargo::rerun-if-env-changed=QMAKE");
        let qmake = std::env::var_os("QMAKE").expect(
            "Building QtBridge for OHOS requires QMAKE to point to the target Qt installation's host-runnable qmake wrapper",
        );
        Some(Self::from_qmake(Path::new(&qmake)))
    }

    fn from_qmake(qmake: &Path) -> Self {
        println!("cargo::rerun-if-changed={}", qmake.display());
        let output = Command::new(qmake)
            .arg("-query")
            .env_clear()
            .output()
            .unwrap_or_else(|err| panic!("Could not run {}: {err}", qmake.display()));
        assert!(
            output.status.success(),
            "{} -query failed: {}",
            qmake.display(),
            String::from_utf8_lossy(&output.stderr),
        );
        let properties = parse_qmake_properties(&String::from_utf8_lossy(&output.stdout));
        for required in ["QT_VERSION", "QT_INSTALL_HEADERS", "QT_INSTALL_LIBS"] {
            assert!(
                properties
                    .get(required)
                    .is_some_and(|value| !value.is_empty()),
                "{} -query is missing {required}",
                qmake.display(),
            );
        }
        Self { properties }
    }

    pub(super) fn query(&self, property: &str) -> String {
        self.properties
            .get(property)
            .unwrap_or_else(|| panic!("Target qmake did not provide {property}"))
            .clone()
    }
}

fn parse_qmake_properties(output: &str) -> BTreeMap<String, String> {
    output
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.to_owned(), value.trim().to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_qmake_properties;

    #[test]
    fn target_and_host_paths_remain_distinct() {
        let properties = parse_qmake_properties(
            "QT_VERSION:6.12.0\nQT_INSTALL_HEADERS:/qt/ohos/include\n\
             QT_INSTALL_LIBS:/qt/ohos/lib\nQT_HOST_BINS:/qt/macos/bin\n",
        );
        assert_eq!(properties["QT_INSTALL_HEADERS"], "/qt/ohos/include");
        assert_eq!(properties["QT_INSTALL_LIBS"], "/qt/ohos/lib");
        assert_eq!(properties["QT_HOST_BINS"], "/qt/macos/bin");
    }

    #[test]
    fn windows_drive_letters_are_preserved() {
        let properties = parse_qmake_properties("QT_INSTALL_LIBS:C:/Qt/6.12.0/lib\r\n");
        assert_eq!(properties["QT_INSTALL_LIBS"], "C:/Qt/6.12.0/lib");
    }
}
