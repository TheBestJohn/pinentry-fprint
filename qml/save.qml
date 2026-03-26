import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import org.kde.kirigami as Kirigami

ApplicationWindow {
    id: root
    title: "Save for Fingerprint Unlock?"
    width: 400
    height: 170
    visible: true
    color: palette.window
    flags: Qt.Dialog | Qt.WindowStaysOnTopHint

    SystemPalette { id: palette }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 12

        RowLayout {
            spacing: 12

            Kirigami.Icon {
                source: "fingerprint-gui"
                implicitWidth: 48
                implicitHeight: 48
            }

            ColumnLayout {
                spacing: 4
                Label {
                    text: "Save passphrase?"
                    font.pixelSize: 18
                    font.bold: true
                    color: palette.windowText
                }
                Label {
                    text: "Store in the system keyring so you can\nunlock with your fingerprint next time."
                    font.pixelSize: 12
                    color: palette.windowText
                    opacity: 0.7
                }
            }
        }

        Item { Layout.fillHeight: true }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: 8

            Button {
                text: "Save"
                highlighted: true
                onClicked: {
                    console.log("RESULT:save")
                    Qt.exit(0)
                }
            }

            Button {
                text: "Don't Save"
                onClicked: {
                    console.log("RESULT:nosave")
                    Qt.exit(1)
                }
            }
        }
    }
}
