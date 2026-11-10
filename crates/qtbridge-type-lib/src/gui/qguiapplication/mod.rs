// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

pub type QGuiApplication = cxx_qt_lib::QGuiApplication;

#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_env = "ohos")]
pub use ohos::new_with_arguments;
