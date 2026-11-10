// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR BSD-3-Clause

extern "C" int qtbridge_main(int argc, char **argv);

// Qt's OHOS platform plugin loads this library and calls main on QtMainThread.
// QGuiApplication and QQmlApplicationEngine are both owned by the Rust QApp.
int main(int argc, char **argv)
{
    return qtbridge_main(argc, argv);
}
