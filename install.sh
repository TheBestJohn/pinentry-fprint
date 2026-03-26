#!/usr/bin/env bash
set -e

PREFIX="${HOME}/.local"
BIN_DIR="${PREFIX}/bin"
DATA_DIR="${PREFIX}/share/pinentry-fprint"
APP_DIR="${PREFIX}/share/applications"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "pinentry-fprint installer"
echo ""
echo "Install to:"
echo "  Binary:   ${BIN_DIR}/pinentry-fprint"
echo "  QML:      ${DATA_DIR}/qml/"
echo "  Desktop:  ${APP_DIR}/pinentry-fprint.desktop"
echo ""
read -p "Continue? [Y/n] " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Nn]$ ]]; then
    echo "Aborted."
    exit 0
fi

mkdir -p "${BIN_DIR}" "${DATA_DIR}/qml" "${APP_DIR}"

cp "${SCRIPT_DIR}/pinentry-fprint" "${BIN_DIR}/pinentry-fprint"
chmod +x "${BIN_DIR}/pinentry-fprint"
echo "  Installed pinentry-fprint -> ${BIN_DIR}/"

cp "${SCRIPT_DIR}"/qml/*.qml "${DATA_DIR}/qml/"
echo "  Installed QML dialogs -> ${DATA_DIR}/qml/"

cp "${SCRIPT_DIR}/pinentry-fprint.desktop" "${APP_DIR}/"
echo "  Installed desktop file -> ${APP_DIR}/"

# Build qml-run if Qt6 dev headers are available
if pkg-config --exists Qt6Gui Qt6Qml 2>/dev/null && [ -f "${SCRIPT_DIR}/qml/qml-run.cpp" ]; then
    echo "  Building qml-run (KDE window icon helper)..."
    g++ -o "${BIN_DIR}/qml-run" "${SCRIPT_DIR}/qml/qml-run.cpp" \
        $(pkg-config --cflags --libs Qt6Gui Qt6Qml) -fPIC 2>/dev/null \
        && echo "  Installed qml-run -> ${BIN_DIR}/" \
        || echo "  Skipped qml-run (build failed, optional)"
else
    echo "  Skipped qml-run (Qt6 dev headers not found, optional)"
fi

# Configure gpg-agent if not already set
AGENT_CONF="${HOME}/.gnupg/gpg-agent.conf"
if [ -f "${AGENT_CONF}" ] && grep -q "pinentry-fprint" "${AGENT_CONF}"; then
    echo "  gpg-agent already configured"
else
    echo ""
    read -p "Configure gpg-agent to use pinentry-fprint? [Y/n] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        mkdir -p "${HOME}/.gnupg"
        if [ -f "${AGENT_CONF}" ]; then
            sed -i '/^pinentry-program/d' "${AGENT_CONF}"
        fi
        echo "pinentry-program ${BIN_DIR}/pinentry-fprint" >> "${AGENT_CONF}"
        echo "  Updated ${AGENT_CONF}"
        gpgconf --kill gpg-agent 2>/dev/null || true
        echo "  Restarted gpg-agent"
    fi
fi

echo ""
echo "Done! Run 'pinentry-fprint --add-keys' to register your GPG keys."
