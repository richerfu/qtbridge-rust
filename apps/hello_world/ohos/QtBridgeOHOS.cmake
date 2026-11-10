# Copyright (C) 2026 The Qt Company Ltd.
# SPDX-License-Identifier: LicenseRef-Qt-Commercial OR BSD-3-Clause

# Local Cargo integration for the hello_world OHOS example.
include_guard(GLOBAL)

# CMake's OHOS platform does not define the built-in WHOLE_ARCHIVE feature.
# Preserve native CXX-Qt initializers when the Cargo archive is linked by lld.
set(CMAKE_CXX_LINK_LIBRARY_USING_QTBRIDGE_RUST
    "LINKER:--whole-archive" "<LINK_ITEM>" "LINKER:--no-whole-archive")
set(CMAKE_CXX_LINK_LIBRARY_USING_QTBRIDGE_RUST_SUPPORTED TRUE)

# Import a Cargo staticlib using the same target Qt and SDK as CMake. Only target
# C/C++ compilation receives SDK flags; build scripts and proc macros stay native.
function(qtbridge_import_ohos_crate target)
    cmake_parse_arguments(PARSE_ARGV 1 arg "" "MANIFEST_PATH;PACKAGE" "")
    if(NOT OHOS OR NOT CMAKE_CROSSCOMPILING)
        message(FATAL_ERROR "Configure with the Qt for HarmonyOS qt-cmake toolchain")
    endif()
    if(NOT arg_MANIFEST_PATH OR NOT arg_PACKAGE)
        message(FATAL_ERROR "qtbridge_import_ohos_crate needs MANIFEST_PATH and PACKAGE")
    endif()
    if(OHOS_ARCH STREQUAL "arm64-v8a")
        set(rust_target aarch64-unknown-linux-ohos)
    elseif(OHOS_ARCH STREQUAL "x86_64")
        set(rust_target x86_64-unknown-linux-ohos)
    else()
        message(FATAL_ERROR "Unsupported QtBridge OHOS architecture: ${OHOS_ARCH}")
    endif()
    find_program(cargo_executable cargo REQUIRED NO_CMAKE_FIND_ROOT_PATH)
    get_target_property(host_qmake Qt6::qmake IMPORTED_LOCATION)
    if(NOT host_qmake OR NOT QT_HOST_PATH OR NOT QT6_INSTALL_PREFIX)
        message(FATAL_ERROR "Qt for HarmonyOS requires target Qt and host Qt tools from find_package(Qt6)")
    endif()

    # Qt6::qmake is intentionally imported from host Qt. Its default qt.conf
    # describes host headers/libraries. Give Cargo a wrapper whose metadata is
    # target Qt, while QT_HOST_* still selects executable tools for this host.
    set(qt_conf "${CMAKE_CURRENT_BINARY_DIR}/qtbridge-target.conf")
    get_filename_component(qt_target_archdata "${QT6_INSTALL_ARCHDATA}" ABSOLUTE
        BASE_DIR "${QT6_INSTALL_PREFIX}")
    # Use the installation layout exported by Qt, including non-default layouts.
    # Do not infer an installation prefix from the location of Qt6Config.cmake.
    string(CONCAT qt_conf_content
        "[Paths]\n"
        "Prefix=${QT6_INSTALL_PREFIX}\n"
        "Headers=${QT6_INSTALL_HEADERS}\n"
        "Libraries=${QT6_INSTALL_LIBS}\n"
        "LibraryExecutables=${QT6_INSTALL_LIBEXECS}\n"
        "Binaries=${QT6_INSTALL_BINS}\n"
        "Plugins=${QT6_INSTALL_PLUGINS}\n"
        "QmlImports=${QT6_INSTALL_QML}\n"
        "ArchData=${QT6_INSTALL_ARCHDATA}\n"
        "Data=${QT6_INSTALL_DATA}\n"
        "HostPrefix=${QT_HOST_PATH}\n"
        "HostBinaries=${QT6_HOST_INFO_BINDIR}\n"
        "HostLibraryExecutables=${QT6_HOST_INFO_LIBEXECDIR}\n"
        "HostLibraries=${QT6_HOST_INFO_LIBDIR}\n"
        "HostData=${qt_target_archdata}\n"
        "TargetSpec=${QT_QMAKE_TARGET_MKSPEC}\n"
        "HostSpec=${QT6_HOST_INFO_QMAKE_MKSPEC}\n"
        "SysrootifyPrefix=false\n"
    )
    # Like Qt's own generated qt.conf, update these only when content changes.
    # An unchanged configure must not invalidate Cargo's Qt discovery.
    file(GENERATE OUTPUT "${qt_conf}" CONTENT "${qt_conf_content}")
    # Cargo watches the qmake wrapper. Include the configuration in its content
    # so changing the target Qt layout or version also invalidates discovery.
    string(SHA256 qt_conf_hash "${Qt6_VERSION}\n${qt_conf_content}")
    if(CMAKE_HOST_WIN32)
        set(target_qmake "${CMAKE_CURRENT_BINARY_DIR}/qtbridge-qmake.bat")
        file(GENERATE OUTPUT "${target_qmake}"
            CONTENT "@rem Qt configuration: ${qt_conf_hash}\r\n@\"${host_qmake}\" -qtconf \"${qt_conf}\" %*\r\n")
    else()
        set(target_qmake "${CMAKE_CURRENT_BINARY_DIR}/qtbridge-qmake")
        string(REPLACE "'" "'\\''" host_qmake_shell "${host_qmake}")
        string(REPLACE "'" "'\\''" qt_conf_shell "${qt_conf}")
        file(GENERATE OUTPUT "${target_qmake}"
            CONTENT "#!/bin/sh\n# Qt configuration: ${qt_conf_hash}\nexec '${host_qmake_shell}' -qtconf '${qt_conf_shell}' \"$@\"\n"
            FILE_PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE GROUP_READ GROUP_EXECUTE WORLD_READ WORLD_EXECUTE)
    endif()

    string(REPLACE "-" "_" rust_target_env "${rust_target}")
    # cc-rs normally splits flags on whitespace. Preserve CMake's quoting and
    # keep a sysroot containing spaces as one compiler argument.
    string(REPLACE "\\" "\\\\" rust_sysroot "${CMAKE_SYSROOT}")
    string(REPLACE "\"" "\\\"" rust_sysroot "${rust_sysroot}")
    set(target_flags "--sysroot=\"${rust_sysroot}\"")
    if(CMAKE_CXX_COMPILER_TARGET)
        string(APPEND target_flags " --target=${CMAKE_CXX_COMPILER_TARGET}")
    endif()
    set(cargo_dir "${CMAKE_CURRENT_BINARY_DIR}/cargo")
    set(profile "$<IF:$<CONFIG:Debug>,debug,release>")
    set(archive "${cargo_dir}/${rust_target}/${profile}/lib${arg_PACKAGE}.a")
    get_filename_component(manifest_path "${arg_MANIFEST_PATH}" ABSOLUTE
        BASE_DIR "${CMAKE_CURRENT_SOURCE_DIR}")
    get_filename_component(crate_dir "${manifest_path}" DIRECTORY)

    # Cargo tracks all Rust, CXX, and embedded QML inputs. Always let it check
    # freshness, with the static archive declared as a byproduct for Ninja.
    add_custom_target(${target}_build
        COMMAND "${CMAKE_COMMAND}" -E env
            "QMAKE=${target_qmake}"
            "CARGO_TARGET_DIR=${cargo_dir}"
            "CC_${rust_target_env}=${CMAKE_C_COMPILER}"
            "CXX_${rust_target_env}=${CMAKE_CXX_COMPILER}"
            "AR_${rust_target_env}=${CMAKE_AR}"
            "CC_SHELL_ESCAPED_FLAGS=1"
            "CFLAGS_${rust_target_env}=${CMAKE_C_FLAGS} ${target_flags}"
            "CXXFLAGS_${rust_target_env}=${CMAKE_CXX_FLAGS} ${target_flags}"
            "${cargo_executable}" build
                --manifest-path "${manifest_path}"
                --package "${arg_PACKAGE}" --lib --target "${rust_target}"
                "$<$<NOT:$<CONFIG:Debug>>:--release>"
        WORKING_DIRECTORY "${crate_dir}"
        BYPRODUCTS "${archive}"
        COMMAND_EXPAND_LISTS
        USES_TERMINAL
        VERBATIM
    )
    add_library(${target} STATIC IMPORTED GLOBAL)
    set_target_properties(${target} PROPERTIES
        IMPORTED_LOCATION "${cargo_dir}/${rust_target}/release/lib${arg_PACKAGE}.a"
        IMPORTED_LOCATION_DEBUG "${cargo_dir}/${rust_target}/debug/lib${arg_PACKAGE}.a"
    )
    add_dependencies(${target} ${target}_build)
endfunction()
