// Copyright (C) 2026 The Qt Company Ltd.
// SPDX-License-Identifier: LicenseRef-Qt-Commercial OR BSD-3-Clause

import QtQuick
import QtQuick.Controls.Fusion
import hello_world

ApplicationWindow {
    width: 480
    height: 640
    visible: true
    title: qsTr("QtBridge Hello World")

    Button {
        objectName: "helloButton"
        anchors.centerIn: parent
        text: "Hello World!"
        onClicked: text = Backend.say_hello()
    }
}
