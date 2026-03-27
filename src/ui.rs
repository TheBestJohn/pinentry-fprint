use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

fn qml_path(name: &str) -> String {
    // Check system-wide install
    let installed = format!("/usr/share/pinentry-fprint/qml/{name}");
    if std::path::Path::new(&installed).exists() {
        return installed;
    }
    // Check user-local install (next to binary)
    if let Ok(exe) = std::env::current_exe()
        && let Some(bin_dir) = exe.parent()
    {
        // ~/.local/bin/../share/pinentry-fprint/qml/
        let user_share = bin_dir.join("../share/pinentry-fprint/qml").join(name);
        if user_share.exists() {
            return user_share.to_string_lossy().into_owned();
        }
    }
    // Dev fallback: source tree
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{manifest}/qml/{name}")
}

#[derive(Debug, Clone, Copy)]
enum Backend {
    Qml,
    Kdialog,
    Zenity,
}

static CACHED_BACKEND: OnceLock<Backend> = OnceLock::new();

fn detect_backend() -> Backend {
    *CACHED_BACKEND.get_or_init(detect_backend_inner)
}

fn detect_backend_inner() -> Backend {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    let is_kde = desktop.contains("kde")
        || desktop.contains("plasma")
        || std::env::var("KDE_SESSION_VERSION").is_ok();

    if is_kde && (which("qml-run") || which("qml6")) {
        return Backend::Qml;
    }

    if is_kde && which("kdialog") {
        return Backend::Kdialog;
    }

    if which("zenity") {
        return Backend::Zenity;
    }

    if which("kdialog") {
        return Backend::Kdialog;
    }

    // Last resort
    Backend::Zenity
}

static CACHED_QML_RUNNER: OnceLock<String> = OnceLock::new();

fn qml_runner() -> &'static str {
    CACHED_QML_RUNNER.get_or_init(|| {
        for path in ["/usr/local/bin/qml-run", "/usr/bin/qml-run"] {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let p = format!("{home}/.local/bin/qml-run");
            if std::path::Path::new(&p).exists() {
                return p;
            }
        }
        "qml6".to_string()
    })
}

fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn show_fingerprint_waiting(
    description: &str,
    key_id: &str,
    attempt: u32,
) -> std::io::Result<Child> {
    let retry_msg = if attempt > 1 {
        "Not recognized - try again\n\n"
    } else {
        ""
    };

    let child = match detect_backend() {
        Backend::Qml => {
            let runner = qml_runner();
            let mut cmd = Command::new(runner);
            cmd.arg(qml_path("fingerprint.qml"));
            if runner == "qml6" {
                cmd.arg("--");
            }
            cmd.args([
                "--desc",
                description,
                "--key",
                key_id,
                "--attempt",
                &attempt.to_string(),
            ]);
            cmd.env("QT_WAYLAND_APP_ID", "pinentry-fprint");
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?
        }
        Backend::Kdialog => Command::new("kdialog")
            .args([
                "--title",
                "GPG Fingerprint Unlock",
                "--yes-label",
                "Use Password Instead",
                "--no-label",
                "Cancel",
                "--warningyesno",
                &format!(
                    "<h2>Touch Fingerprint Sensor</h2>\
                    <p style='color: #e74c3c;'>{retry_msg}</p>\
                    <p>{description}</p>\
                    <p><small>Key: {key_id}</small></p>\
                    <br/><p><i>Waiting for fingerprint...</i></p>"
                ),
            ])
            .spawn()?,
        Backend::Zenity => {
            let plain = format!(
                "Touch Fingerprint Sensor\n\n\
                {retry_msg}\
                {description}\n\
                Key: {key_id}\n\n\
                Waiting for fingerprint..."
            );
            let mut cmd = Command::new("zenity");
            cmd.arg("--info")
                .arg("--title=GPG Fingerprint Unlock")
                .arg("--icon=fingerprint-gui")
                .arg(format!("--text={plain}"))
                .arg("--no-markup")
                .arg("--ok-label=Use Password Instead")
                .arg("--extra-button=Cancel")
                .spawn()?
        }
    };
    Ok(child)
}

pub fn show_password_dialog(_description: &str, _key_id: &str) -> Option<String> {
    read_password("GPG Passphrase")
}

pub fn show_password_dialog_with_error(_description: &str, _key_id: &str) -> Option<String> {
    read_password("GPG Passphrase - Wrong password, try again")
}

fn read_password(title: &str) -> Option<String> {
    let output = match detect_backend() {
        Backend::Qml | Backend::Kdialog => Command::new("kdialog")
            .args(["--title", title, "--password", "Enter passphrase:"])
            .output()
            .ok()?,
        Backend::Zenity => Command::new("zenity")
            .args([
                "--password",
                &format!("--title={title}"),
                "--window-icon=fingerprint-gui",
            ])
            .output()
            .ok()?,
    };

    if output.status.success() {
        let pass = String::from_utf8_lossy(&output.stdout)
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();
        if pass.is_empty() { None } else { Some(pass) }
    } else {
        None
    }
}

pub fn ask_save_to_keyring() -> bool {
    match detect_backend() {
        Backend::Qml => Command::new(qml_runner())
            .arg(qml_path("save.qml"))
            .env("QT_WAYLAND_APP_ID", "pinentry-fprint")
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        Backend::Kdialog => Command::new("kdialog")
            .args([
                                "--title", "Save for Fingerprint Unlock?",
                "--yes-label", "Save",
                "--no-label", "Don't Save",
                "--yesno",
                "<h3>Save passphrase?</h3>\
                <p>Store in the system keyring so you can<br/>unlock with your fingerprint next time.</p>",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        Backend::Zenity => Command::new("zenity")
            .args([
                "--info",
                "--title=Save for Fingerprint Unlock?",
                "--icon=fingerprint-gui",
                "--text=Save passphrase?\n\nStore in the system keyring so you can\nunlock with your fingerprint next time.",
                "--no-markup",
                "--ok-label=Save",
                "--extra-button=Don't Save",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
    }
}
