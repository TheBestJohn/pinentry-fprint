use zbus::Connection;

const FPRINT_BUS: &str = "net.reactivated.Fprint";
const MANAGER_PATH: &str = "/net/reactivated/Fprint/Manager";
const MANAGER_IFACE: &str = "net.reactivated.Fprint.Manager";
const DEVICE_IFACE: &str = "net.reactivated.Fprint.Device";

pub enum VerifyResult {
    Match,
    NoMatch,
    #[allow(dead_code)]
    Error(String),
}

pub async fn verify(username: &str) -> VerifyResult {
    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(e) => return VerifyResult::Error(format!("D-Bus connect: {e}")),
    };

    // Get default device path
    let device_path: zbus::zvariant::OwnedObjectPath = match conn
        .call_method(
            Some(FPRINT_BUS),
            MANAGER_PATH,
            Some(MANAGER_IFACE),
            "GetDefaultDevice",
            &(),
        )
        .await
    {
        Ok(reply) => match reply.body().deserialize() {
            Ok(p) => p,
            Err(e) => return VerifyResult::Error(format!("GetDefaultDevice parse: {e}")),
        },
        Err(e) => return VerifyResult::Error(format!("GetDefaultDevice: {e}")),
    };
    // Subscribe to VerifyStatus signal BEFORE starting verify
    let dbus_proxy = match zbus::fdo::DBusProxy::new(&conn).await {
        Ok(p) => p,
        Err(e) => return VerifyResult::Error(format!("DBusProxy: {e}")),
    };

    let rule = match zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface(DEVICE_IFACE)
        .and_then(|b| b.member("VerifyStatus"))
    {
        Ok(b) => b.build(),
        Err(e) => return VerifyResult::Error(format!("MatchRule: {e}")),
    };
    if let Err(e) = dbus_proxy.add_match_rule(rule).await {
        return VerifyResult::Error(format!("AddMatch: {e}"));
    }

    let mut stream = { zbus::MessageStream::from(&conn) };

    // Now claim and start verify
    if let Err(e) = claim(&conn, &device_path, username).await {
        return VerifyResult::Error(format!("Claim: {e}"));
    }

    if let Err(e) = conn
        .call_method(
            Some(FPRINT_BUS),
            &device_path,
            Some(DEVICE_IFACE),
            "VerifyStart",
            &("any"),
        )
        .await
    {
        let _ = release(&conn, &device_path).await;
        return VerifyResult::Error(format!("VerifyStart: {e}"));
    }

    // Wait for result
    use futures_lite::StreamExt;
    let result = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while let Some(Ok(msg)) = stream.next().await {
            let msg: zbus::Message = msg;
            let header = msg.header();
            if header.member().map(|m| m.as_str()) == Some("VerifyStatus")
                && let Ok((status, _done)) = msg.body().deserialize::<(String, bool)>()
            {
                let _ = stop(&conn, &device_path).await;
                return if status == "verify-match" {
                    VerifyResult::Match
                } else {
                    VerifyResult::NoMatch
                };
            }
        }
        VerifyResult::Error("Stream ended".into())
    })
    .await;

    match result {
        Ok(r) => r,
        Err(_) => {
            let _ = stop(&conn, &device_path).await;
            VerifyResult::Error("Timeout".into())
        }
    }
}

async fn claim(
    conn: &Connection,
    device_path: &zbus::zvariant::OwnedObjectPath,
    username: &str,
) -> Result<(), zbus::Error> {
    conn.call_method(
        Some(FPRINT_BUS),
        device_path,
        Some(DEVICE_IFACE),
        "Claim",
        &(username),
    )
    .await?;
    Ok(())
}

async fn release(
    conn: &Connection,
    device_path: &zbus::zvariant::OwnedObjectPath,
) -> Result<(), zbus::Error> {
    conn.call_method(
        Some(FPRINT_BUS),
        device_path,
        Some(DEVICE_IFACE),
        "Release",
        &(),
    )
    .await?;
    Ok(())
}

async fn stop(
    conn: &Connection,
    device_path: &zbus::zvariant::OwnedObjectPath,
) -> Result<(), zbus::Error> {
    let _ = conn
        .call_method(
            Some(FPRINT_BUS),
            device_path,
            Some(DEVICE_IFACE),
            "VerifyStop",
            &(),
        )
        .await;
    release(conn, device_path).await
}
