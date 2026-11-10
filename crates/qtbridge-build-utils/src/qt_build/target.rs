// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use qt_build_utils::{QtInstallation as _, QtInstallationQMake};

pub(super) struct TargetQtInstallation {
    properties: BTreeMap<String, String>,
}

impl TargetQtInstallation {
    pub(super) fn discover(expected_version: &str) -> Option<Self> {
        let cross_compiling = std::env::var_os("HOST")
            .zip(std::env::var_os("TARGET"))
            .is_some_and(|(host, target)| host != target);
        if !cross_compiling {
            return None;
        }
        println!("cargo::rerun-if-env-changed=QMAKE");
        if let Some(qmake) = std::env::var_os("QMAKE") {
            return Some(
                Self::from_qmake(Path::new(&qmake))
                    .unwrap_or_else(|error| panic!("Could not query target Qt: {error}")),
            );
        }

        // Match QtBuild's qmake search order and version. QMAKE is optional for
        // cross builds, just as it is for native builds.
        println!("cargo::rerun-if-env-changed=PATH");
        ["qmake6", "qmake-qt5", "qmake"]
            .into_iter()
            .filter_map(|name| which::which(name).ok())
            .find_map(|qmake| {
                let installation = QtInstallationQMake::try_from(qmake.clone()).ok()?;
                if installation.version().to_string() != expected_version {
                    return None;
                }
                Some(
                    Self::from_qmake(&qmake)
                        .unwrap_or_else(|error| panic!("Could not query target Qt: {error}")),
                )
            })
    }

    fn from_qmake(qmake: &Path) -> Result<Self, TargetQtError> {
        println!("cargo::rerun-if-changed={}", qmake.display());
        let output = Command::new(qmake)
            .arg("-query")
            .env_clear()
            .output()
            .map_err(|source| TargetQtError::Execute(qmake.to_owned(), source))?;
        if !output.status.success() {
            return Err(TargetQtError::Failed(
                qmake.to_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        let properties = parse_qmake_properties(&String::from_utf8_lossy(&output.stdout));
        for required in ["QT_VERSION", "QT_INSTALL_HEADERS", "QT_INSTALL_LIBS"] {
            if properties.get(required).is_none_or(String::is_empty) {
                return Err(TargetQtError::MissingProperty(qmake.to_owned(), required));
            }
        }
        Ok(Self { properties })
    }

    pub(super) fn query(&self, property: &str) -> String {
        self.properties
            .get(property)
            .unwrap_or_else(|| panic!("Target qmake did not provide {property}"))
            .clone()
    }
}

#[derive(Debug)]
enum TargetQtError {
    Execute(PathBuf, std::io::Error),
    Failed(PathBuf, String),
    MissingProperty(PathBuf, &'static str),
}

impl fmt::Display for TargetQtError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execute(path, error) => {
                write!(formatter, "could not run {}: {error}", path.display())
            }
            Self::Failed(path, stderr) => {
                write!(formatter, "{} -query failed: {stderr}", path.display())
            }
            Self::MissingProperty(path, property) => {
                write!(formatter, "{} -query is missing {property}", path.display())
            }
        }
    }
}

impl std::error::Error for TargetQtError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Execute(_, error) => Some(error),
            _ => None,
        }
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
