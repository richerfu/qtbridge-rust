// Copyright (C) 2025 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR LGPL-3.0-only

use super::QGuiApplication;

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qguiapplication.h");
        include!("cxx-qt-lib/core/qvector/qvector_QByteArray.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;
        type QVector_QByteArray = cxx_qt_lib::QVector<cxx_qt_lib::QByteArray>;

        #[namespace = "rust::cxxqtlib1"]
        fn qguiapplicationNew(args: &QVector_QByteArray) -> UniquePtr<QGuiApplication>;
    }
}

/// Constructs the application with owned arguments from a native launcher.
/// The argument storage is retained by the application for its entire lifetime.
pub fn new_with_arguments(arguments: &[Vec<u8>]) -> cxx::UniquePtr<QGuiApplication> {
    let mut args = cxx_qt_lib::QVector::<cxx_qt_lib::QByteArray>::default();
    for argument in arguments {
        assert!(
            !argument.contains(&0),
            "Qt application arguments must not contain NUL"
        );
        args.append(cxx_qt_lib::QByteArray::from(argument.as_slice()));
    }
    assert!(
        !arguments.is_empty(),
        "Qt requires an application name in argv[0]"
    );
    ffi::qguiapplicationNew(&args)
}
