import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.kde.kirigami as Kirigami

ApplicationWindow {
    id: root
    title: "GPG Fingerprint Unlock"
    width: 440
    height: 360
    visible: true
    color: palette.window
    flags: Qt.Dialog | Qt.WindowStaysOnTopHint

    function getArg(name, fallback) {
        var args = Qt.application.arguments
        var idx = args.indexOf(name)
        return idx !== -1 && idx + 1 < args.length ? args[idx + 1] : fallback
    }

    property string description: getArg("--desc", "")
    property string keyId: getArg("--key", "")
    property int attempt: parseInt(getArg("--attempt", "1"), 10)

    // Exit codes: 0 = password, 1 = cancel, timeout handled by Rust killing process
    signal usePassword()
    signal cancel()

    SystemPalette { id: palette }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 12

        RowLayout {
            spacing: 16
            Layout.alignment: Qt.AlignLeft

            Kirigami.Icon {
                source: "fingerprint-gui"
                implicitWidth: 64
                implicitHeight: 64
                Layout.alignment: Qt.AlignTop
            }

            ColumnLayout {
                spacing: 6

                Label {
                    text: "Touch Fingerprint Sensor"
                    font.pixelSize: 20
                    font.bold: true
                    color: palette.windowText
                }

                Label {
                    visible: root.attempt > 1
                    text: "Not recognized - try again"
                    font.pixelSize: 13
                    color: Kirigami.Theme.negativeTextColor
                    font.bold: true
                }

                Label {
                    text: root.description
                    font.pixelSize: 12
                    color: palette.windowText
                    opacity: 0.8
                    wrapMode: Text.WordWrap
                    Layout.maximumWidth: 280
                }

                Label {
                    visible: root.keyId !== ""
                    text: "Key: " + root.keyId
                    font.pixelSize: 10
                    color: palette.windowText
                    opacity: 0.5
                    elide: Text.ElideMiddle
                    Layout.maximumWidth: 280
                }
            }
        }

        Item { Layout.fillHeight: true }

        // Pulsing indicator
        Label {
            text: "Waiting for fingerprint..."
            font.pixelSize: 13
            font.italic: true
            color: palette.highlight
            Layout.alignment: Qt.AlignHCenter

            SequentialAnimation on opacity {
                loops: Animation.Infinite
                NumberAnimation { from: 1.0; to: 0.3; duration: 800 }
                NumberAnimation { from: 0.3; to: 1.0; duration: 800 }
            }
        }

        Item { Layout.preferredHeight: 8 }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Use Password Instead"
                onClicked: {
                    console.log("RESULT:password")
                    Qt.exit(0)
                }
            }

            Button {
                text: "Cancel"
                onClicked: {
                    console.log("RESULT:cancel")
                    Qt.exit(1)
                }
            }
        }
    }
}
