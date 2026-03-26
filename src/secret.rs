use tokio::io::AsyncWriteExt;

pub async fn store(key_id: &str, passphrase: &str) -> Result<(), String> {
    let mut child = tokio::process::Command::new("secret-tool")
        .args([
            "store",
            "--label",
            &format!("GPG Passphrase ({key_id})"),
            "application",
            "pinentry-fprint",
            "key-id",
            key_id,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn secret-tool: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(passphrase.as_bytes())
            .await
            .map_err(|e| format!("write: {e}"))?;
    }

    let status = child.wait().await.map_err(|e| format!("wait: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("secret-tool store failed".into())
    }
}

pub async fn lookup(key_id: &str) -> Option<String> {
    let output = tokio::process::Command::new("secret-tool")
        .args(["lookup", "application", "pinentry-fprint", "key-id", key_id])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
        if s.is_empty() { None } else { Some(s) }
    } else {
        None
    }
}
