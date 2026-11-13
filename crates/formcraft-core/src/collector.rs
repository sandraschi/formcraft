//! Opt-in public response collector. Loopback HTTP, per-form token, truncated address.
//! A filled honeypot is discarded and stored nowhere.

use formcraft_store::Store;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

const MAX_BODY: usize = 65_536;

#[derive(Debug, PartialEq, Eq)]
pub enum PublicDecision {
    Stored { id: i64 },
    Discarded,
    Unauthorized,
    UnknownForm,
    BadRequest,
}

/// IPv4 keeps the /24. IPv6 keeps the /64. An address that does not parse is dropped.
pub fn truncate_peer(peer: &str) -> Option<String> {
    let ip: IpAddr = peer.parse().ok()?;
    match ip {
        IpAddr::V4(v4) => Some(v4_net(v4)),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return Some(v4_net(v4));
            }
            let s = v6.segments();
            Some(format!("{:x}:{:x}:{:x}:{:x}::", s[0], s[1], s[2], s[3]))
        }
    }
}

fn v4_net(v4: Ipv4Addr) -> String {
    let o = v4.octets();
    format!("{}.{}.{}.0", o[0], o[1], o[2])
}

fn tokens_match(expected: &str, given: &str) -> bool {
    let a = expected.as_bytes();
    let b = given.as_bytes();
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn accept_public(
    store: &mut Store,
    form_id: &str,
    token: &str,
    answers: &Value,
    nps_score: Option<i64>,
    account_id: Option<&str>,
    honeypot: &str,
) -> Result<PublicDecision, String> {
    if !honeypot.trim().is_empty() {
        return Ok(PublicDecision::Discarded);
    }
    match store.get_form(form_id) {
        Ok(_) => {}
        Err(formcraft_store::FormError::NotFound(_)) => return Ok(PublicDecision::UnknownForm),
        Err(e) => return Err(e.to_string()),
    }
    let expected = store.collect_token(form_id).map_err(|e| e.to_string())?;
    let Some(expected) = expected else {
        return Ok(PublicDecision::Unauthorized);
    };
    if !tokens_match(&expected, token) {
        return Ok(PublicDecision::Unauthorized);
    }
    if !answers.is_object() {
        return Ok(PublicDecision::BadRequest);
    }
    if let Some(score) = nps_score {
        if !(0..=10).contains(&score) {
            return Ok(PublicDecision::BadRequest);
        }
    }
    if account_id.is_some_and(|a| a.len() > 64) {
        return Ok(PublicDecision::BadRequest);
    }
    let sentiment = crate::estimate_sentiment(&answers.to_string());
    let id = store
        .add_collected_response(form_id, account_id, answers, nps_score, sentiment, None)
        .map_err(|e| e.to_string())?;
    Ok(PublicDecision::Stored { id })
}

pub fn accept_public_from_peer(
    store: &mut Store,
    form_id: &str,
    token: &str,
    answers: &Value,
    nps_score: Option<i64>,
    account_id: Option<&str>,
    honeypot: &str,
    peer_ip: &str,
) -> Result<PublicDecision, String> {
    let net = truncate_peer(peer_ip);
    let decision = accept_public(
        store, form_id, token, answers, nps_score, account_id, honeypot,
    )?;
    if let PublicDecision::Stored { id } = decision {
        store
            .set_submitter_net(id, net.as_deref())
            .map_err(|e| e.to_string())?;
        return Ok(PublicDecision::Stored { id });
    }
    Ok(decision)
}

pub fn bind_collector(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))
}

pub fn serve_collector(db: &Path, listener: TcpListener) -> Result<(), String> {
    loop {
        serve_one(db, &listener)?;
    }
}

pub fn serve_one(db: &Path, listener: &TcpListener) -> Result<(), String> {
    let (mut stream, peer) = listener.accept().map_err(|e| e.to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let ip = peer.ip().to_string();
    if let Err(status) = handle(&mut stream, db, &ip) {
        let _ = write_json(&mut stream, status.0, status.1, status.2);
    }
    Ok(())
}

fn handle(
    stream: &mut TcpStream,
    db: &Path,
    peer_ip: &str,
) -> Result<(), (u16, &'static str, &'static str)> {
    let incoming = read_http(stream).map_err(|_| (400, "Bad Request", "{\"stored\":false}"))?;
    if incoming.method == "GET" && incoming.path == "/health" {
        write_json(stream, 200, "OK", "{\"ok\":true}")?;
        return Ok(());
    }
    if incoming.method == "OPTIONS" {
        write_json(stream, 204, "No Content", "")?;
        return Ok(());
    }
    if incoming.method != "POST" {
        return Err((404, "Not Found", "{\"stored\":false}"));
    }
    let form_id =
        form_id_from_path(&incoming.path).ok_or((404, "Not Found", "{\"stored\":false}"))?;
    let body: Value = serde_json::from_slice(&incoming.body)
        .map_err(|_| (400, "Bad Request", "{\"stored\":false}"))?;
    let honeypot = match body.get("company_website") {
        None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(_) => "filled".to_string(),
    };
    let token = body.get("token").and_then(Value::as_str).unwrap_or("");
    let answers = body.get("answers").cloned().unwrap_or(Value::Null);
    let nps = body.get("nps_score").and_then(Value::as_i64);
    if body.get("nps_score").is_some() && nps.is_none() {
        return Err((400, "Bad Request", "{\"stored\":false}"));
    }
    let account = body.get("account_id").and_then(Value::as_str);
    let mut store = Store::open(db).map_err(|_| (500, "Error", "{\"stored\":false}"))?;
    let decision = accept_public_from_peer(
        &mut store, &form_id, token, &answers, nps, account, &honeypot, peer_ip,
    )
    .map_err(|_| (500, "Error", "{\"stored\":false}"))?;
    match decision {
        PublicDecision::Stored { id } => {
            let body = json!({"stored": true, "id": id}).to_string();
            write_owned(stream, 200, "OK", &body)
        }
        PublicDecision::Discarded => write_json(stream, 200, "OK", "{\"stored\":false}"),
        PublicDecision::Unauthorized => {
            write_json(stream, 401, "Unauthorized", "{\"stored\":false}")
        }
        PublicDecision::UnknownForm => write_json(stream, 404, "Not Found", "{\"stored\":false}"),
        PublicDecision::BadRequest => write_json(stream, 400, "Bad Request", "{\"stored\":false}"),
    }
}

struct Incoming {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_http(stream: &mut TcpStream) -> Result<Incoming, String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        if buf.len() > 8192 {
            return Err("headers".into());
        }
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("closed".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = find_sep(&buf) {
            break i;
        }
    };
    let header = std::str::from_utf8(&buf[..header_end]).map_err(|e| e.to_string())?;
    let mut lines = header.split("\r\n");
    let request = lines.next().ok_or("empty")?;
    let mut parts = request.split(' ');
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut content_length: Option<usize> = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().ok();
        }
    }
    let mut body = buf[header_end + 4..].to_vec();
    if method == "POST" {
        let need = content_length.ok_or("length")?;
        if need > MAX_BODY {
            return Err("large".into());
        }
        while body.len() < need {
            let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("short".into());
            }
            body.extend_from_slice(&tmp[..n]);
        }
        body.truncate(need);
    }
    Ok(Incoming { method, path, body })
}

fn find_sep(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn form_id_from_path(path: &str) -> Option<String> {
    let path = path.split('?').next().unwrap_or(path);
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() != 5 || parts[1] != "v1" || parts[2] != "forms" || parts[4] != "responses" {
        return None;
    }
    let id = percent_decode(parts[3])?;
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return None;
    }
    Some(id)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn write_json(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    body: &str,
) -> Result<(), (u16, &'static str, &'static str)> {
    write_owned(stream, code, reason, body)
}

fn write_owned(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    body: &str,
) -> Result<(), (u16, &'static str, &'static str)> {
    let bytes = body.as_bytes();
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n",
        bytes.len()
    );
    stream
        .write_all(head.as_bytes())
        .map_err(|_| (500, "Error", "{\"stored\":false}"))?;
    stream
        .write_all(bytes)
        .map_err(|_| (500, "Error", "{\"stored\":false}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};

    const FORM: &str = "nps-q4";
    const GOOD: &str = "test-token-value-1";

    fn ready(store: &mut Store) {
        store
            .save_form(FORM, "Q4", "survey", &json!({"rating": "integer"}))
            .unwrap();
        store.set_collect_token(FORM, GOOD).unwrap();
    }

    #[test]
    fn public_collect_truncates_the_address_and_drops_the_honeypot() {
        let mut store = Store::in_memory().unwrap();
        ready(&mut store);
        let stored = accept_public_from_peer(
            &mut store,
            FORM,
            GOOD,
            &json!({"comment": "gut"}),
            Some(9),
            None,
            "",
            "203.0.113.45",
        )
        .unwrap();
        assert_eq!(stored, PublicDecision::Stored { id: 1 });
        let rows = store.list_responses(FORM).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].submitter_net.as_deref(), Some("203.0.113.0"));
        let blob = format!("{} {:?}", rows[0].answers, rows[0].submitter_net);
        assert!(!blob.contains("203.0.113.45"));

        let mapped = truncate_peer("::ffff:198.51.100.20").unwrap();
        assert_eq!(mapped, "198.51.100.0");
        let v6 = truncate_peer("2001:db8:1:2:3:4:5:6").unwrap();
        assert_eq!(v6, "2001:db8:1:2::");
        assert!(truncate_peer("203.0.113.45:80").is_none());

        let denied = accept_public_from_peer(
            &mut store,
            FORM,
            "wrong-token-value-x",
            &json!({"comment": "x"}),
            Some(1),
            None,
            "",
            "203.0.113.45",
        )
        .unwrap();
        assert_eq!(denied, PublicDecision::Unauthorized);
        let dropped = accept_public_from_peer(
            &mut store,
            FORM,
            GOOD,
            &json!({"comment": "spam"}),
            Some(0),
            None,
            "https://spam.example",
            "203.0.113.45",
        )
        .unwrap();
        assert_eq!(dropped, PublicDecision::Discarded);
        assert_eq!(store.list_responses(FORM).unwrap().len(), 1);
        let short = store.set_collect_token(FORM, "too-short");
        assert!(short.is_err());
    }

    #[test]
    fn serve_one_stores_a_posted_response_on_loopback() {
        let dir = std::env::temp_dir().join(format!("formcraft-collect-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("formcraft.db");
        let _ = std::fs::remove_file(&db);
        {
            let mut store = Store::open(&db).unwrap();
            ready(&mut store);
        }
        let listener = bind_collector(0).unwrap();
        let port = listener.local_addr().unwrap().port();
        let db2 = db.clone();
        let worker = std::thread::spawn(move || serve_one(&db2, &listener));
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let body = format!(
            "{{\"token\":\"{GOOD}\",\"answers\":{{\"comment\":\"gut\"}},\"nps_score\":9,\"company_website\":\"\"}}"
        );
        let req = format!(
            "POST /v1/forms/{FORM}/responses HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(req.as_bytes()).unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        let text = String::from_utf8_lossy(&buf);
        worker.join().unwrap().unwrap();
        assert!(text.starts_with("HTTP/1.1 200"), "{text}");
        assert!(text.contains("\"stored\":true"), "{text}");
        assert!(!text.contains(GOOD), "{text}");
        let store = Store::open(&db).unwrap();
        let rows = store.list_responses(FORM).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].submitter_net.as_deref(), Some("127.0.0.0"));
        let blob = format!("{:?}", rows[0].submitter_net);
        assert!(!blob.contains("127.0.0.1"));
    }
}
