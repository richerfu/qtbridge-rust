// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR BSD-3-Clause

#![cfg(target_env = "ohos")]

use qtbridge::{qobject, QApp};

#[derive(Default)]
pub struct Backend {}

#[qobject(Singleton)]
impl Backend {
    #[qslot]
    fn say_hello(&self) -> String {
        println!("Hello from Rust!");
        "Hello from Rust!".to_owned()
    }
}

fn run_application(mut app: QApp) -> i32 {
    app.register::<Backend>()
        .load_qml(include_bytes!("Main.qml"))
        .run()
}

/// Entry called by the Qt OHOS application library on QtMainThread.
///
/// # Safety
/// `argv` must contain `argc` valid pointers to NUL-terminated strings, as
/// supplied to main by Qt. The application must not have been created already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qtbridge_main(
    argc: std::ffi::c_int,
    argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    if argc <= 0 || argv.is_null() {
        return 2;
    }
    // A native library entry does not initialize Rust argv. Copy Qt's arguments
    // into storage that QGuiApplication retains for its entire lifetime.
    let pointers = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    if pointers.iter().any(|pointer| pointer.is_null()) {
        return 2;
    }
    let arguments: Vec<Vec<u8>> = pointers
        .iter()
        .map(|pointer| {
            unsafe { std::ffi::CStr::from_ptr(*pointer) }
                .to_bytes()
                .to_vec()
        })
        .collect();
    // Never unwind across the native main boundary.
    std::panic::catch_unwind(|| run_application(QApp::with_arguments(&arguments))).unwrap_or(1)
}
