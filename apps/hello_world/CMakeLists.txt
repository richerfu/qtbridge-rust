# Copyright (C) 2026 The Qt Company Ltd.
# SPDX-License-Identifier: LicenseRef-Qt-Commercial OR BSD-3-Clause

cmake_minimum_required(VERSION 3.25)
project(QtBridgeHelloWorld LANGUAGES C CXX)

find_package(Qt6 6.12 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2 Test QuickTest)
qt_standard_project_setup()

include("${CMAKE_CURRENT_LIST_DIR}/ohos/QtBridgeOHOS.cmake")
qtbridge_import_ohos_crate(hello_world_rust
    MANIFEST_PATH "${CMAKE_CURRENT_LIST_DIR}/ohos/Cargo.toml"
    PACKAGE hello_world
)

# Qt creates a MODULE library with an exported main on OHOS, and supplies the
# Ability/Stage, XComponent integration, and the hello_world_make_hap target.
qt_add_executable(hello_world ohos/main.cpp)
# harmonydeployqt also deploys the QML roots. Stage only the OHOS example's QML.
set(qml_deploy_dir "${CMAKE_CURRENT_BINARY_DIR}/qml")
file(MAKE_DIRECTORY "${qml_deploy_dir}")
configure_file(ohos/src/Main.qml "${qml_deploy_dir}/Main.qml" COPYONLY)
target_link_libraries(hello_world PRIVATE
    "$<LINK_LIBRARY:QTBRIDGE_RUST,hello_world_rust>"
    Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::QuickControls2
    Qt6::Test Qt6::QuickTest
    dl m
)
set_target_properties(hello_world PROPERTIES
    BUILD_WITH_INSTALL_RPATH TRUE
    INSTALL_RPATH "$ORIGIN"
    QT_HARMONYOS_APP_BUNDLE_NAME "org.qtproject.qtbridge.hello"
    QT_HARMONYOS_APP_NAME "QtBridge Hello World"
    QT_HARMONYOS_APP_LABEL "QtBridge Hello World"
    QT_HARMONYOS_MODULE_DEVICE_TYPES "phone;tablet;2in1"
    QT_QML_ROOT_PATH "${qml_deploy_dir}"
)
