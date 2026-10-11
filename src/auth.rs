//! Terminal client for the existing Termleaf Cloud browser/device login flow.

mod session;

use std::io::{self, IsTerminal, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use session::SessionStore;

const DEFAULT_ORIGIN: &str = "https://api.termleaf.com";
const MAX_RESPONSE: u64 = 65_536;

struct Client {
    origin: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Flow {
    flow_id: String,
    poll_token: String,
    authorization_url: String,
    pairing_code: String,
    expires_at: String,
}

#[derive(Serialize, Deserialize)]
struct Session {
    origin: String,
    access_token: String,
    expires_at: u64,
}

impl Client {
    fn configured() -> Result<Self, String> {
        let origin = match std::env::var("TERMLEAF_CLOUD_URL") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => DEFAULT_ORIGIN.into(),
            Err(_) => return Err("TERMLEAF_CLOUD_URL must be valid UTF-8.".into()),
        };
        Ok(Self {
            origin: validate_origin(&origin)?,
        })
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> Result<(u16, Value), String> {
        // Curl configuration arrives via stdin so neither bearer nor poll tokens
        // appear in the process list. Ignore ~/.curlrc and never follow redirects.
        let mut config = format!(
            "url = {}\nrequest = {}\nheader = \"Accept: application/json\"\n",
            curl_quote(&format!("{}/v1{path}", self.origin)),
            curl_quote(method)
        );
        if let Some(token) = token {
            if !valid_secret(token) {
                return Err("Invalid saved session; run 'termleaf logout --local'.".into());
            }
            config.push_str(&format!(
                "header = {}\n",
                curl_quote(&format!("Authorization: Bearer {token}"))
            ));
        }
        if let Some(body) = body {
            config.push_str(&format!(
                "header = \"Content-Type: application/json\"\ndata-binary = {}\n",
                curl_quote(&body.to_string())
            ));
        }
        let protocol = if self.origin.starts_with("https:") {
            "=https"
        } else {
            "=http"
        };
        let mut child = Command::new("curl")
            .args([
                "-q",
                "--config",
                "-",
                "--silent",
                "--proto",
                protocol,
                "--tlsv1.2",
                "--connect-timeout",
                "5",
                "--max-time",
                "15",
                "--max-filesize",
                "65536",
                "--write-out",
                "\n%{http_code}",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Cannot run curl. Install curl to use cloud authentication.".to_owned())?;
        let write_result = child.stdin.take().unwrap().write_all(config.as_bytes());
        let mut output = Vec::new();
        let read_result = child
            .stdout
            .take()
            .unwrap()
            .take(MAX_RESPONSE + 8)
            .read_to_end(&mut output);
        if write_result.is_err() || read_result.is_err() || output.len() as u64 > MAX_RESPONSE + 4 {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cloud response could not be read or exceeded the size limit.".into());
        }
        let status = child
            .wait()
            .map_err(|_| "Cannot finish the cloud request.".to_owned())?;
        if !status.success() {
            return Err("Cannot reach Termleaf Cloud. Check your connection, system clock and TLS certificates, then try again.".into());
        }
        let split = output
            .iter()
            .rposition(|byte| *byte == b'\n')
            .ok_or("Invalid cloud HTTP response.")?;
        let code = std::str::from_utf8(&output[split + 1..])
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or("Invalid cloud HTTP status.")?;
        if (300..400).contains(&code) {
            return Err("Cloud redirects are not accepted. Check TERMLEAF_CLOUD_URL.".into());
        }
        let value = serde_json::from_slice(&output[..split]).unwrap_or(Value::Null);
        Ok((code, value))
    }

    fn start(&self, device_name: &str) -> Result<Flow, String> {
        let (code, body) = self.request(
            "POST",
            "/auth/google/start",
            Some(json!({"deviceName": device_name})),
            None,
        )?;
        require_status(code, &[200], &body)?;
        let flow: Flow =
            serde_json::from_value(body).map_err(|_| "Invalid cloud login response.".to_owned())?;
        validate_flow(&flow, &self.origin)?;
        Ok(flow)
    }

    fn complete(&self, flow: &Flow) -> Result<Option<(Session, String)>, String> {
        let (code, body) =
            self.request("POST", "/auth/google/complete", Some(flow_body(flow)), None)?;
        require_status(code, &[200, 202], &body)?;
        if code == 202 && body.get("status").and_then(Value::as_str) == Some("pending") {
            return Ok(None);
        }
        if code != 200 {
            return Err("Invalid cloud login response.".into());
        }
        let token = body
            .get("accessToken")
            .and_then(Value::as_str)
            .filter(|token| valid_secret(token))
            .ok_or("Invalid cloud session response.")?;
        let expires_at = parse_timestamp(
            body.get("expiresAt")
                .and_then(Value::as_str)
                .ok_or("Missing session expiry.")?,
        )?;
        if expires_at <= now_ms() {
            return Err("The new session has already expired. Check your system clock.".into());
        }
        let email = account_email(&body)?;
        Ok(Some((
            Session {
                origin: self.origin.clone(),
                access_token: token.into(),
                expires_at,
            },
            email,
        )))
    }

    fn cancel(&self, flow: &Flow) {
        let _ = self.request("POST", "/auth/google/cancel", Some(flow_body(flow)), None);
    }
}

// Also revokes a session if approval raced a timeout, cancellation or disk error.
struct PendingLogin<'a> {
    client: &'a Client,
    flow: Flow,
    saved: bool,
}
impl Drop for PendingLogin<'_> {
    fn drop(&mut self) {
        if !self.saved {
            self.client.cancel(&self.flow);
        }
    }
}

pub(crate) fn login(open_browser: bool, device_name: Option<&str>) -> Result<(), String> {
    let client = Client::configured()?;
    let store = SessionStore::new().map_err(storage_error)?;
    let _lock = store.lock().map_err(storage_error)?;
    if store.load().map_err(storage_error)?.is_some() {
        return Err("A session is already saved. Use 'termleaf whoami' or 'termleaf logout' before signing in again.".into());
    }
    let default_name = format!("Termleaf terminal ({})", std::env::consts::OS);
    let flow = client.start(device_name.unwrap_or(&default_name))?;
    let mut pending = PendingLogin {
        client: &client,
        flow,
        saved: false,
    };
    println!(
        "Confirm this device in your browser with code: {}",
        pending.flow.pairing_code
    );
    println!("Open: {}", pending.flow.authorization_url);
    println!("Sign in with Google and enter the code shown above. Press Ctrl+C or Esc to cancel.");
    io::stdout()
        .flush()
        .map_err(|_| "Cannot display the login instructions.".to_owned())?;
    if open_browser && !launch_browser(&pending.flow.authorization_url) {
        println!("Could not open a browser automatically. Open the URL above on any device.");
    }
    let terminal = LoginTerminal::new()?;
    let result = wait_for_session(&client, &pending.flow, || terminal.canceled());
    drop(terminal);
    let (session, email) = result?;
    let bytes =
        serde_json::to_vec(&session).map_err(|_| "Cannot encode the session.".to_owned())?;
    store.save(&bytes).map_err(storage_error)?;
    pending.saved = true;
    println!("Signed in as {email}. Use 'termleaf whoami' to check your subscription.");
    Ok(())
}

fn wait_for_session(
    client: &Client,
    flow: &Flow,
    mut canceled: impl FnMut() -> Result<bool, String>,
) -> Result<(Session, String), String> {
    let remaining = parse_timestamp(&flow.expires_at)?
        .saturating_sub(now_ms())
        .min(300_000);
    let deadline = Instant::now() + Duration::from_millis(remaining);
    loop {
        if canceled()? {
            return Err("Login canceled.".into());
        }
        if Instant::now() >= deadline {
            return Err("Login timed out. Run 'termleaf login' again.".into());
        }
        let session = client.complete(flow)?;
        if canceled()? {
            return Err("Login canceled.".into());
        }
        if Instant::now() >= deadline {
            return Err("Login timed out. Run 'termleaf login' again.".into());
        }
        if let Some(session) = session {
            return Ok(session);
        }
        let next = (Instant::now() + Duration::from_secs(2)).min(deadline);
        while Instant::now() < next {
            if canceled()? {
                return Err("Login canceled.".into());
            }
            std::thread::sleep(
                Duration::from_millis(100).min(next.saturating_duration_since(Instant::now())),
            );
        }
    }
}

pub(crate) fn logout(local_only: bool) -> Result<(), String> {
    let store = SessionStore::new().map_err(storage_error)?;
    let _lock = store.lock().map_err(storage_error)?;
    if local_only {
        // Recovery must also work for corrupt/oversized/non-private session files.
        // clear removes this directory entry without reading or following it.
        store.clear().map_err(storage_error)?;
        println!("Local session removed. The server session was not revoked.");
        return Ok(());
    }
    let Some(bytes) = store.load().map_err(storage_error)? else {
        println!("Not signed in.");
        return Ok(());
    };
    let session = decode_session(&bytes)?;
    let client = session_client(&session)?;
    let (code, body) = client.request("POST", "/auth/logout", None, Some(&session.access_token))?;
    // Expired/revoked sessions have nothing left to revoke.
    require_status(code, &[200, 401], &body).map_err(|e| {
        format!("{e} Your local session is retained; use --local to forget it without revoking it.")
    })?;
    if code == 200 && body.get("ok") != Some(&Value::Bool(true)) {
        return Err("Invalid logout response; local session retained.".into());
    }
    store.clear().map_err(storage_error)?;
    println!("Signed out.");
    Ok(())
}

pub(crate) fn whoami() -> Result<(), String> {
    let store = SessionStore::new().map_err(storage_error)?;
    let bytes = store
        .load()
        .map_err(storage_error)?
        .ok_or("Not signed in. Run 'termleaf login'.")?;
    let session = decode_session(&bytes)?;
    if session.expires_at <= now_ms() {
        return Err(
            "Session expired. Run 'termleaf logout --local', then 'termleaf login'.".into(),
        );
    }
    let client = session_client(&session)?;
    let (code, body) = client.request("GET", "/account", None, Some(&session.access_token))?;
    require_status(code, &[200], &body)?;
    let email = account_email(&body)?;
    let status = body
        .pointer("/subscription/status")
        .and_then(Value::as_str)
        .filter(|s| {
            matches!(
                *s,
                "active"
                    | "trialing"
                    | "inactive"
                    | "canceled"
                    | "expired"
                    | "past_due"
                    | "unpaid"
                    | "incomplete"
                    | "incomplete_expired"
            )
        })
        .ok_or("Invalid subscription response.")?;
    println!("Signed in as {email}\nSubscription: {status}");
    Ok(())
}

fn session_client(session: &Session) -> Result<Client, String> {
    let client = Client::configured()?;
    if client.origin != session.origin {
        return Err("Saved session belongs to another cloud service. Set TERMLEAF_CLOUD_URL to that service, or use 'termleaf logout --local'.".into());
    }
    Ok(client)
}

fn decode_session(bytes: &[u8]) -> Result<Session, String> {
    let session: Session = serde_json::from_slice(bytes)
        .map_err(|_| "Saved session is invalid; run 'termleaf logout --local'.".to_owned())?;
    if !valid_secret(&session.access_token) || validate_origin(&session.origin)? != session.origin {
        return Err("Saved session is invalid; run 'termleaf logout --local'.".into());
    }
    Ok(session)
}

fn storage_error(error: io::Error) -> String {
    if error.kind() == io::ErrorKind::WouldBlock {
        return "Another login/logout command is running. Finish or cancel it first.".into();
    }
    "Cannot securely read or save the cloud session in your user configuration folder. Check its ownership, permissions and available disk space.".into()
}

fn account_email(body: &Value) -> Result<String, String> {
    let email = body
        .pointer("/user/email")
        .and_then(Value::as_str)
        .filter(|s| {
            !s.is_empty() && s.len() <= 254 && s.contains('@') && !s.chars().any(char::is_control)
        })
        .ok_or("Invalid cloud account response.")?;
    Ok(email.into())
}

fn require_status(code: u16, allowed: &[u16], body: &Value) -> Result<(), String> {
    if allowed.contains(&code) {
        return Ok(());
    }
    // Do not echo server-controlled messages or bodies, which can contain secrets
    // or terminal control sequences.
    let reason = match (code, body.pointer("/error/code").and_then(Value::as_str)) {
        (401, _) => "Session expired or revoked. Sign in again.",
        (410, _) => "Login request expired or was already used. Start a new login.",
        (429, _) => "Too many requests. Wait a minute and try again.",
        (_, Some("google_unavailable")) => "Google sign-in is not configured on this service.",
        (_, Some("pairing_failed")) => "Device confirmation failed. Start a new login.",
        (_, Some("google_denied" | "google_login_failed")) => {
            "Google sign-in was canceled or failed. Try again."
        }
        _ => "Termleaf Cloud could not complete the request. Try again.",
    };
    Err(format!("{reason} (HTTP {code})"))
}

fn flow_body(flow: &Flow) -> Value {
    json!({"flowId": flow.flow_id, "pollToken": flow.poll_token})
}
fn valid_secret(value: &str) -> bool {
    value.len() == 43 && safe_identifier(value)
}
fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn validate_origin(value: &str) -> Result<String, String> {
    let origin = value.strip_suffix('/').unwrap_or(value);
    let (scheme, authority) = origin
        .split_once("://")
        .ok_or("Cloud URL must be an HTTPS origin.")?;
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    let valid_host = !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        });
    let valid_port = port.is_none_or(|port| {
        !port.is_empty()
            && port.bytes().all(|b| b.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|p| p > 0)
    });
    if !valid_host || !valid_port || !(scheme == "https" || scheme == "http" && host == "localhost")
    {
        return Err("Cloud URL must be an HTTPS origin without credentials, path, query or fragment; HTTP is allowed only for localhost development.".into());
    }
    Ok(origin.into())
}

fn validate_flow(flow: &Flow, origin: &str) -> Result<(), String> {
    let invalid = || "Invalid cloud login response.".to_owned();
    if !safe_identifier(&flow.flow_id)
        || flow.flow_id.len() > 128
        || !valid_secret(&flow.poll_token)
        || flow.pairing_code.len() != 6
        || !flow.pairing_code.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    let query = flow
        .authorization_url
        .strip_prefix(&format!("{origin}/v1/auth/google/browser?"))
        .ok_or_else(invalid)?;
    let mut id = None;
    let mut state = None;
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("flowId", value)) if id.is_none() => id = Some(value),
            Some(("state", value)) if state.is_none() => state = Some(value),
            _ => return Err(invalid()),
        }
    }
    if id != Some(flow.flow_id.as_str())
        || !state.is_some_and(valid_secret)
        || parse_timestamp(&flow.expires_at)? <= now_ms()
    {
        return Err(invalid());
    }
    Ok(())
}

fn curl_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// Server timestamps are UTC ISO 8601 with optional millisecond precision.
fn parse_timestamp(value: &str) -> Result<u64, String> {
    let invalid = || "Invalid cloud expiry timestamp.".to_owned();
    let b = value.as_bytes();
    if !value.is_ascii()
        || !matches!(b.len(), 20 | 24)
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[b.len() - 1] != b'Z'
        || (b.len() == 24 && b[19] != b'.')
    {
        return Err(invalid());
    }
    let number = |start, end| {
        let part = &value[start..end];
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid());
        }
        part.parse::<i64>().map_err(|_| invalid())
    };
    let (year, month, day, hour, minute, second) = (
        number(0, 4)?,
        number(5, 7)?,
        number(8, 10)?,
        number(11, 13)?,
        number(14, 16)?,
        number(17, 19)?,
    );
    let millis = if b.len() == 24 { number(20, 23)? } else { 0 };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return Err(invalid()),
    };
    if !(1970..=9999).contains(&year)
        || !(1..=days_in_month).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..60).contains(&second)
        || !(0..1000).contains(&millis)
    {
        return Err(invalid());
    }
    let y = year - i64::from(month <= 2);
    let era = y / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + day - 1 - 719468;
    Ok(((days * 86400 + hour * 3600 + minute * 60 + second) * 1000 + millis) as u64)
}

fn launch_browser(url: &str) -> bool {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(target_os = "windows") {
        ("rundll32.exe", vec!["url.dll,FileProtocolHandler", url])
    } else if cfg!(target_os = "android") {
        ("termux-open-url", vec![url])
    } else {
        ("xdg-open", vec![url])
    };
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

struct LoginTerminal(bool);
impl LoginTerminal {
    fn new() -> Result<Self, String> {
        let active = io::stdin().is_terminal() && io::stdout().is_terminal();
        if active {
            crossterm::terminal::enable_raw_mode().map_err(|_| {
                "Cannot watch login cancellation keys. Try a normal terminal.".to_owned()
            })?;
        }
        Ok(Self(active))
    }
    fn canceled(&self) -> Result<bool, String> {
        if !self.0 {
            return Ok(false);
        }
        while event::poll(Duration::ZERO).map_err(|_| "Cannot read terminal input.".to_owned())? {
            if let Event::Key(key) =
                event::read().map_err(|_| "Cannot read terminal input.".to_owned())?
            {
                if key.kind != KeyEventKind::Release
                    && (key.code == KeyCode::Esc
                        || key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
impl Drop for LoginTerminal {
    fn drop(&mut self) {
        if self.0 {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn flow(origin: &str) -> Flow {
        Flow {
            flow_id: "test-flow".into(),
            poll_token: "p".repeat(43),
            authorization_url: format!(
                "{origin}/v1/auth/google/browser?flowId=test-flow&state={}",
                "s".repeat(43)
            ),
            pairing_code: "123456".into(),
            expires_at: "2099-01-01T00:00:00.000Z".into(),
        }
    }

    fn session_response() -> Value {
        json!({"accessToken":"t".repeat(43),"expiresAt":"2099-01-01T00:00:00.000Z","user":{"email":"writer@example.com"},"subscription":{"status":"inactive"}})
    }

    struct Mock {
        client: Client,
        requests: Arc<Mutex<Vec<String>>>,
        thread: Option<std::thread::JoinHandle<()>>,
    }
    impl Mock {
        fn new(responses: Vec<(u16, Value)>) -> Self {
            let listener = TcpListener::bind("localhost:0").unwrap();
            let client = Client {
                origin: format!("http://localhost:{}", listener.local_addr().unwrap().port()),
            };
            listener.set_nonblocking(true).unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let saved = requests.clone();
            let thread = std::thread::spawn(move || {
                for (code, value) in responses {
                    let deadline = Instant::now() + Duration::from_secs(8);
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                assert!(Instant::now() < deadline, "mock request timed out");
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            Err(e) => panic!("{e}"),
                        }
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 4096];
                    loop {
                        let n = stream.read(&mut buffer).unwrap();
                        assert!(n > 0);
                        bytes.extend_from_slice(&buffer[..n]);
                        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&bytes[..end]);
                            let len = headers
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .and_then(|s| s.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if bytes.len() >= end + 4 + len {
                                break;
                            }
                        }
                    }
                    saved
                        .lock()
                        .unwrap()
                        .push(String::from_utf8(bytes).unwrap());
                    let body = value.to_string();
                    write!(stream,"HTTP/1.1 {code} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nLocation: https://other.example/\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                }
            });
            Self {
                client,
                requests,
                thread: Some(thread),
            }
        }
    }
    impl Drop for Mock {
        fn drop(&mut self) {
            let result = self.thread.take().unwrap().join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }

    #[test]
    fn restricts_service_origin_and_browser_handoff() {
        assert_eq!(
            validate_origin("https://api.termleaf.com/").unwrap(),
            DEFAULT_ORIGIN
        );
        assert!(validate_origin("http://localhost:8787").is_ok());
        for url in [
            "http://api.termleaf.com",
            "http://127.0.0.1:8787",
            "https://user@api.termleaf.com",
            "https://api.termleaf.com/path",
            "https://api.termleaf.com?query",
            "https://api.termleaf.com#fragment",
            "https://api.termleaf.com:0",
            "https://api.termleaf.com:65536",
            "https://api.termleaf.com\n",
            "https://-invalid.com",
            "https://api.termleaf.com//",
        ] {
            assert!(validate_origin(url).is_err(), "{url}");
        }
        let mut f = flow(DEFAULT_ORIGIN);
        assert!(validate_flow(&f, DEFAULT_ORIGIN).is_ok());
        for url in [
            format!(
                "https://attacker.example/v1/auth/google/browser?flowId=test-flow&state={}",
                "s".repeat(43)
            ),
            format!(
                "{DEFAULT_ORIGIN}/v1/auth/google/browser?flowId=test-flow&state={}&state=extra",
                "s".repeat(43)
            ),
            format!(
                "{DEFAULT_ORIGIN}/v1/auth/google/browser?flowId=wrong&state={}",
                "s".repeat(43)
            ),
            format!(
                "{DEFAULT_ORIGIN}/v1/auth/google/browser?flowId=test-flow&state={}#secret",
                "s".repeat(43)
            ),
        ] {
            f.authorization_url = url;
            assert!(validate_flow(&f, DEFAULT_ORIGIN).is_err());
        }
    }

    #[test]
    fn validates_expiry_timestamps_and_secret_shapes() {
        assert_eq!(parse_timestamp("1970-01-01T00:00:00Z").unwrap(), 0);
        assert_eq!(
            parse_timestamp("2000-01-01T00:00:00.123Z").unwrap(),
            946684800123
        );
        assert!(parse_timestamp("2024-02-29T23:59:59Z").is_ok());
        for s in [
            "2025-02-29T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:60Z",
            "2026-01-01T00:00:00+09:00",
            "not a timestamp",
            "",
        ] {
            assert!(parse_timestamp(s).is_err(), "{s}");
        }
        assert!(valid_secret(&"a".repeat(43)));
        assert!(!valid_secret(&"a".repeat(42)));
        assert!(!valid_secret(&format!("{}\n", "a".repeat(42))));
    }

    #[test]
    fn sends_json_and_polls_pending_then_accepts_session() {
        let mock = Mock::new(vec![
            (202, json!({"status":"pending"})),
            (200, session_response()),
        ]);
        let f = flow(&mock.client.origin);
        let (session, email) = wait_for_session(&mock.client, &f, || Ok(false)).unwrap();
        assert_eq!(email, "writer@example.com");
        assert_eq!(session.origin, mock.client.origin);
        let requests = mock.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        for req in requests.iter() {
            assert!(req.starts_with("POST /v1/auth/google/complete HTTP/1.1"));
            let body: Value = serde_json::from_str(req.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(body["pollToken"], f.poll_token);
            assert_eq!(body["flowId"], "test-flow");
        }
    }

    #[test]
    fn cancels_a_session_that_races_user_cancellation() {
        let mock = Mock::new(vec![(200, session_response()), (200, json!({"ok":true}))]);
        let pending = PendingLogin {
            client: &mock.client,
            flow: flow(&mock.client.origin),
            saved: false,
        };
        let mut polls = 0;
        let result = wait_for_session(&mock.client, &pending.flow, || {
            polls += 1;
            Ok(polls > 1)
        });
        assert_eq!(result.err().unwrap(), "Login canceled.");
        drop(pending);
        assert!(mock.requests.lock().unwrap()[1].starts_with("POST /v1/auth/google/cancel "));
    }

    #[test]
    fn rejects_redirects_and_redacts_server_errors() {
        let mock = Mock::new(vec![
            (302, json!({"secret":"do not print"})),
            (
                503,
                json!({"error":{"code":"google_unavailable","message":"do not print\u{1b}[2J"}}),
            ),
        ]);
        assert!(mock
            .client
            .request("POST", "/auth/google/start", None, None)
            .err()
            .unwrap()
            .contains("redirects"));
        let error = mock.client.start("terminal").err().unwrap();
        assert!(error.contains("not configured"));
        assert!(!error.contains("do not print"));
        assert!(!error.contains('\u{1b}'));
    }

    #[test]
    fn refuses_invalid_success_responses_and_expired_flows() {
        let mock = Mock::new(vec![
            (200, json!({"accessToken":"short"})),
            (202, json!({"status":"unexpected"})),
        ]);
        let mut f = flow(&mock.client.origin);
        assert!(mock.client.complete(&f).is_err());
        assert!(mock.client.complete(&f).is_err());
        f.expires_at = "2000-01-01T00:00:00Z".into();
        assert!(validate_flow(&f, &mock.client.origin).is_err());
        assert!(wait_for_session(&mock.client, &f, || Ok(false))
            .err()
            .unwrap()
            .contains("timed out"));
    }

    #[test]
    fn escapes_curl_config_without_exposing_bearer_tokens_in_urls() {
        assert_eq!(curl_quote("a\"b\\c\nd\r"), "\"a\\\"b\\\\c\\nd\\r\"");
        let mock = Mock::new(vec![(200, json!({"ok":true}))]);
        mock.client
            .request("POST", "/auth/logout", None, Some(&"t".repeat(43)))
            .unwrap();
        let req = mock.requests.lock().unwrap();
        assert!(req[0].contains(&format!("Authorization: Bearer {}", "t".repeat(43))));
        assert!(req[0].starts_with("POST /v1/auth/logout HTTP/1.1"));
    }
}
