# pinentry-fprint

A GPG pinentry that lets you unlock your keys with a fingerprint sensor.

Uses fprintd over D-Bus for fingerprint verification, with passphrases stored in the system keyring via libsecret. Falls back to a standard password dialog if fingerprint fails or isn't available.

> **DISCLAIMER:** This software is provided "as is", without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and noninfringement. This project has **not been independently audited, penetration tested, or formally reviewed for security vulnerabilities**. It makes no claims regarding its fitness for securing sensitive data. Use at your own risk.

## Security Model

This tool trades passphrase entry for fingerprint verification. It does **not** replace GPG's encryption — your private keys remain encrypted on disk as usual. Here's how it handles your passphrase:

- **Storage:** Your GPG passphrase is stored in the system keyring (kwallet on KDE, gnome-keyring on GNOME) via libsecret's `secret-tool`. The keyring is encrypted and typically unlocked at login via PAM.
- **Retrieval:** When GPG needs your passphrase, pinentry-fprint retrieves it from the keyring only after a successful fingerprint match from fprintd.
- **Fallback:** If fingerprint verification fails or times out, you can enter the passphrase manually. Optionally, the entered passphrase can be saved to the keyring for future fingerprint use.
- **Memory:** Passphrases are zeroed from memory after use via the `zeroize` crate. This reduces the window where a passphrase could be extracted from a memory dump or swap. Note: passphrases retrieved from `secret-tool` pass through the OS process boundary and standard `String` allocation before being zeroized — they are not pinned or mlock'd.
- **Transport:** The passphrase is passed to gpg-agent via the standard Assuan protocol over stdin/stdout (same as any pinentry). It is never written to disk, logged, or transmitted over a network.
- **Trust boundary:** Security depends on fprintd (fingerprint matching), libsecret (keyring storage), and your system's login session security. A compromised user session means a compromised keyring.

## Features

- Fingerprint authentication via fprintd
- Auto-retry on failed fingerprint reads
- Passphrase stored in system keyring (kwallet/gnome-keyring)
- Auto-registers new keys on first password entry
- Multi-key support with `--add-keys` interactive setup
- Native UI for KDE (QML) and GNOME (zenity)
- kdialog fallback for KDE without QML
- Desktop file for proper window icon on Wayland

## Requirements

- Rust 1.80+
- fprintd with enrolled fingerprints
- libsecret / secret-tool
- One of: qml6 (Qt6), zenity, or kdialog
- Optional: Qt6 development headers (for qml-run icon helper)

## Install

Download the latest release for your architecture from [Releases](https://github.com/TheBestJohn/pinentry-fprint/releases) and run the installer:

```sh
# Download and extract (x86_64 example, also available: aarch64, armv7, riscv64)
mkdir -p /tmp/pinentry-fprint
curl -L https://github.com/TheBestJohn/pinentry-fprint/releases/latest/download/pinentry-fprint-x86_64-linux.tar.gz | tar xz -C /tmp/pinentry-fprint

# Run installer
/tmp/pinentry-fprint/install.sh

# Clean up
rm -rf /tmp/pinentry-fprint
```

The installer will:
- Copy the binary to `~/.local/bin/`
- Install QML dialogs for KDE/Plasma
- Install the desktop file for Wayland window icons
- Build the QML icon helper if Qt6 dev headers are present
- Optionally configure gpg-agent

### Build from source

```sh
cargo build --release
./install.sh
```

## Configure GPG

Point gpg-agent to use pinentry-fprint:

```sh
# ~/.gnupg/gpg-agent.conf
pinentry-program /home/YOUR_USER/.local/bin/pinentry-fprint
```

Then restart the agent:

```sh
gpgconf --kill gpg-agent
```

## Setup Keys

Interactive key registration (lists your GPG keys, stores passphrase for all keygrips):

```sh
pinentry-fprint --add-keys
```

Or manually for specific keygrips:

```sh
pinentry-fprint --setup KEYGRIP1 KEYGRIP2 ...
```

## How It Works

1. gpg-agent calls pinentry-fprint via the Assuan protocol
2. pinentry-fprint looks up the passphrase in the system keyring by keygrip
3. If found, shows a fingerprint dialog and verifies via fprintd
4. On match, returns the cached passphrase to gpg-agent
5. On failure, offers retry or password entry fallback
6. If no cached passphrase exists, shows password dialog and offers to save

## UI Backends

| Desktop | Fingerprint Dialog | Password Dialog |
|---------|-------------------|-----------------|
| KDE/Plasma | QML (native Qt) | kdialog |
| GNOME | zenity | zenity |
| Other | zenity or kdialog | zenity or kdialog |

## License

MIT
