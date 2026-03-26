#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QIcon>

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setWindowIcon(QIcon::fromTheme("fingerprint-gui"));
    app.setApplicationName("pinentry-fprint");
    app.setDesktopFileName("pinentry-fprint");

    QQmlApplicationEngine engine;
    engine.load(QUrl::fromLocalFile(argv[1]));
    if (engine.rootObjects().isEmpty()) return 1;
    return app.exec();
}
