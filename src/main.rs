mod assuan;
mod fingerprint;
mod secret;
mod ui;

use assuan::{AssuanServer, Command};
use zeroize::Zeroize;

const MAX_FINGERPRINT_ATTEMPTS: u32 = 5;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--setup") => {
            let key_ids: Vec<&str> = if args.len() > 2 {
                args[2..].iter().map(|s| s.as_str()).collect()
            } else {
                vec!["default"]
            };
            run_setup(&key_ids);
            return;
        }
        Some("--add-keys") => {
            run_add_keys();
            return;
        }
        Some("--help" | "-h") => {
            println!("pinentry-fprint - GPG pinentry with fingerprint support");
            println!();
            println!("Usage:");
            println!("  pinentry-fprint              Run as pinentry (called by gpg-agent)");
            println!("  pinentry-fprint --add-keys   Interactive: pick GPG keys to register");
            println!("  pinentry-fprint --setup <keygrip...>  Store passphrase for keygrips");
            return;
        }
        _ => {}
    }

    let mut server = AssuanServer::new();
    server.send_greeting();

    let mut description = String::new();
    let mut key_id = String::new();
    let mut had_error = false;

    while let Some(cmd) = server.read_command() {
        match cmd {
            Command::SetDesc(d) => {
                description = d;
                server.send_ok();
            }
            Command::SetError(_) => {
                had_error = true;
                server.send_ok();
            }
            Command::SetKeyInfo(info) => {
                key_id = info
                    .strip_prefix("n/")
                    .or_else(|| info.strip_prefix("s/"))
                    .unwrap_or(&info)
                    .to_string();
                server.send_ok();
            }
            Command::Reset => {
                description.clear();
                key_id.clear();
                had_error = false;
                server.send_ok();
            }
            Command::GetPin => {
                let lookup_key = if key_id.is_empty() {
                    "default"
                } else {
                    &key_id
                };

                match handle_getpin(&description, lookup_key, had_error) {
                    Some(mut passphrase) => {
                        server.send_data(&passphrase);
                        server.send_ok();
                        passphrase.zeroize();
                    }
                    None => {
                        server.send_error(83886179, "Operation cancelled");
                    }
                }
                had_error = false;
            }
            Command::GetInfo(what) => match what.as_str() {
                "flavor" => {
                    server.send_data("pinentry-fprint");
                    server.send_ok();
                }
                "version" => {
                    server.send_data(env!("CARGO_PKG_VERSION"));
                    server.send_ok();
                }
                "ttyinfo" => {
                    server.send_data("- - -");
                    server.send_ok();
                }
                "pid" => {
                    server.send_data(&std::process::id().to_string());
                    server.send_ok();
                }
                _ => server.send_ok(),
            },
            Command::Confirm | Command::Message => server.send_ok(),
            Command::Bye => {
                server.send_ok();
                break;
            }
            Command::Other => server.send_ok(),
        }
    }
}

fn handle_getpin(description: &str, key_id: &str, was_wrong: bool) -> Option<String> {
    let username = std::env::var("SUDO_USER")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "root".into());

    let rt = tokio::runtime::Runtime::new().ok()?;

    if was_wrong {
        let passphrase = ui::show_password_dialog_with_error(description, key_id)?;
        if ui::ask_save_to_keyring() {
            let _ = rt.block_on(secret::store(key_id, &passphrase));
            let _ = rt.block_on(secret::store("default", &passphrase));
        }
        return Some(passphrase);
    }

    let cached = rt
        .block_on(secret::lookup(key_id))
        .or_else(|| rt.block_on(secret::lookup("default")));

    if let Some(ref passphrase) = cached {
        match try_fingerprint_loop(&username, passphrase, description, key_id) {
            FpLoopResult::Matched(p) => return Some(p),
            FpLoopResult::UsePassword => {}
            FpLoopResult::Cancelled => return None,
        }
    }

    let passphrase = ui::show_password_dialog(description, key_id)?;

    if ui::ask_save_to_keyring() {
        let _ = rt.block_on(secret::store(key_id, &passphrase));
        let _ = rt.block_on(secret::store("default", &passphrase));
    }

    Some(passphrase)
}

enum FpLoopResult {
    Matched(String),
    UsePassword,
    Cancelled,
}

fn try_fingerprint_loop(
    username: &str,
    cached_passphrase: &str,
    description: &str,
    key_id: &str,
) -> FpLoopResult {
    let mut attempt = 0u32;
    loop {
        attempt = attempt.saturating_add(1);

        if attempt > MAX_FINGERPRINT_ATTEMPTS {
            return FpLoopResult::UsePassword;
        }

        let mut dialog = match ui::show_fingerprint_waiting(description, key_id, attempt) {
            Ok(child) => child,
            Err(_) => return FpLoopResult::UsePassword,
        };

        let user = username.to_string();
        let fp_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(15),
                    fingerprint::verify(&user),
                )
                .await
            })
        });

        loop {
            if let Ok(Some(status)) = dialog.try_wait() {
                std::thread::spawn(move || {
                    let _ = fp_handle.join();
                });
                if status.success() {
                    return FpLoopResult::UsePassword;
                } else {
                    return FpLoopResult::Cancelled;
                }
            }

            if fp_handle.is_finished() {
                let _ = dialog.kill();
                let _ = dialog.wait();
                match fp_handle.join().ok() {
                    Some(Ok(fingerprint::VerifyResult::Match)) => {
                        return FpLoopResult::Matched(cached_passphrase.to_string());
                    }
                    Some(Ok(fingerprint::VerifyResult::NoMatch)) => {
                        break; // retry
                    }
                    _ => return FpLoopResult::UsePassword,
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

fn run_setup(key_ids: &[&str]) {
    use std::io::{self, Write};

    println!("pinentry-fprint setup");
    println!(
        "Storing passphrase for {} key(s) in the system keyring.",
        key_ids.len()
    );
    println!();

    print!("Enter GPG passphrase: ");
    io::stdout().flush().unwrap();

    let mut passphrase = rpassword::read_password().unwrap_or_else(|_| {
        eprintln!("Failed to read passphrase");
        std::process::exit(1);
    });

    if passphrase.is_empty() {
        eprintln!("Empty passphrase, aborting.");
        std::process::exit(1);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    for key_id in key_ids {
        match rt.block_on(secret::store(key_id, &passphrase)) {
            Ok(()) => println!("  Stored for {key_id}"),
            Err(e) => eprintln!("  Failed for {key_id}: {e}"),
        }
    }

    let _ = rt.block_on(secret::store("default", &passphrase));
    passphrase.zeroize();

    println!();
    println!("Restart gpg-agent: gpgconf --kill gpg-agent");
}

fn run_add_keys() {
    use std::io::{self, BufRead, Write};

    let output = match std::process::Command::new("gpg")
        .args(["--list-secret-keys", "--with-keygrip", "--with-colons"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to run gpg: {e}");
            return;
        }
    };

    let text = String::from_utf8_lossy(&output.stdout);

    struct KeyInfo {
        uid: String,
        fingerprint: String,
        keygrips: Vec<String>,
    }

    let mut keys: Vec<KeyInfo> = Vec::new();
    let mut current: Option<KeyInfo> = None;

    for line in text.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        match fields.first() {
            Some(&"sec") => {
                if let Some(k) = current.take() {
                    keys.push(k);
                }
                current = Some(KeyInfo {
                    uid: String::new(),
                    fingerprint: String::new(),
                    keygrips: Vec::new(),
                });
            }
            Some(&"fpr") => {
                if let Some(ref mut k) = current
                    && k.fingerprint.is_empty()
                {
                    k.fingerprint = fields.get(9).unwrap_or(&"").to_string();
                }
            }
            Some(&"uid") => {
                if let Some(ref mut k) = current
                    && k.uid.is_empty()
                {
                    k.uid = fields.get(9).unwrap_or(&"").to_string();
                }
            }
            Some(&"grp") => {
                if let Some(ref mut k) = current
                    && let Some(grip) = fields.get(9)
                    && !grip.is_empty()
                {
                    k.keygrips.push(grip.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(k) = current.take() {
        keys.push(k);
    }

    if keys.is_empty() {
        println!("No secret keys found.");
        return;
    }

    println!("pinentry-fprint: Register keys for fingerprint unlock\n");
    for (i, key) in keys.iter().enumerate() {
        println!(
            "  [{}] {} ({}... | {} subkey(s))",
            i + 1,
            key.uid,
            &key.fingerprint[..16.min(key.fingerprint.len())],
            key.keygrips.len().saturating_sub(1),
        );
    }
    println!("  [a] All keys");
    println!("  [q] Quit");
    println!();
    print!("Select: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input).unwrap();
    let input = input.trim();

    let selected: Vec<&KeyInfo> = match input {
        "q" | "Q" => return,
        "a" | "A" => keys.iter().collect(),
        n => {
            if let Ok(idx) = n.parse::<usize>() {
                if idx >= 1 && idx <= keys.len() {
                    vec![&keys[idx - 1]]
                } else {
                    eprintln!("Invalid selection.");
                    return;
                }
            } else {
                eprintln!("Invalid selection.");
                return;
            }
        }
    };

    let all_grips: Vec<&str> = selected
        .iter()
        .flat_map(|k| k.keygrips.iter().map(|g| g.as_str()))
        .collect();

    if all_grips.is_empty() {
        println!("No keygrips found for selected keys.");
        return;
    }

    println!(
        "\nRegistering {} keygrip(s) for {} key(s).\n",
        all_grips.len(),
        selected.len()
    );

    print!("Enter passphrase: ");
    io::stdout().flush().unwrap();

    let mut passphrase = rpassword::read_password().unwrap_or_else(|_| {
        eprintln!("Failed to read passphrase");
        std::process::exit(1);
    });

    if passphrase.is_empty() {
        eprintln!("Empty passphrase, aborting.");
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    for grip in &all_grips {
        match rt.block_on(secret::store(grip, &passphrase)) {
            Ok(()) => println!("  Stored for {grip}"),
            Err(e) => eprintln!("  Failed for {grip}: {e}"),
        }
    }
    let _ = rt.block_on(secret::store("default", &passphrase));
    passphrase.zeroize();

    println!("\nDone. Restart gpg-agent: gpgconf --kill gpg-agent");
}
