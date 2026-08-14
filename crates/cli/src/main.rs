//! EcoQuest operator CLI. All data access goes through public HTTPS API routes.
use clap::{Args, Parser, Subcommand};
use reqwest::{
    header::{COOKIE, SET_COOKIE},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(name = "ecoquest", version, about = "EcoQuest API operator CLI")]
struct Cli {
    #[arg(
        long,
        env = "ECOQUEST_API_URL",
        default_value = "http://localhost:8080"
    )]
    api_url: String,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand, Debug)]
enum Command {
    /// Query API and database health. Requires no session.
    Health,
    Login(Login),
    Me,
    Organizations {
        #[command(subcommand)]
        command: OrganizationCommand,
    },
    Events {
        #[command(subcommand)]
        command: EventCommand,
    },
    Participants {
        event_id: String,
    },
    Verify {
        event_id: String,
        participant_id: String,
        #[arg(long)]
        yes: bool,
    },
    Certificates {
        #[command(subcommand)]
        command: CertificateCommand,
    },
    Stats,
}
#[derive(Args, Debug)]
struct Login {
    #[arg(long)]
    email: Option<String>,
    #[arg(long)]
    password: Option<String>,
}
#[derive(Subcommand, Debug)]
enum OrganizationCommand {
    Status,
}
#[derive(Subcommand, Debug)]
enum CertificateCommand {
    IssueStatus { participation_id: String },
}
#[derive(Subcommand, Debug)]
enum EventCommand {
    Create(CreateEvent),
    List,
    GenerateQr { event_id: String },
}
#[derive(Args, Debug)]
struct CreateEvent {
    #[arg(long)]
    organization_id: String,
    #[arg(long)]
    name: String,
    #[arg(long, default_value = "")]
    description: String,
    #[arg(long)]
    activity_type: String,
    #[arg(long)]
    location: String,
    #[arg(long)]
    starts_at: String,
    #[arg(long)]
    ends_at: String,
    #[arg(long)]
    capacity: i32,
    #[arg(long)]
    eco_points: i32,
}
/// Session cookie names set by the API. These must match `ACCESS_COOKIE` and
/// `REFRESH_COOKIE` in `crates/api/src/auth/extract.rs`; if they drift, login
/// silently keeps no cookies and every later command reports "not logged in".
const ACCESS_COOKIE: &str = "eq_access";
const REFRESH_COOKIE: &str = "eq_refresh";

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    api_url: String,
    cookie: String,
}
fn session_path() -> Result<PathBuf, String> {
    let root = env::var_os("APPDATA")
        .or_else(|| env::var_os("HOME"))
        .ok_or("cannot find APPDATA or HOME for session storage")?;
    Ok(PathBuf::from(root).join("EcoQuest").join("session.json"))
}
fn save_session(session: &Session) -> Result<(), String> {
    let path = session_path()?;
    let parent = path.parent().ok_or("invalid session path")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create session directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("cannot secure session directory: {e}"))?;
    }
    let data = serde_json::to_vec(session).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| format!("cannot write session: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("cannot secure session file: {e}"))?;
    }
    Ok(())
}
fn load_session(api_url: &str) -> Result<Session, String> {
    let raw =
        fs::read(session_path()?).map_err(|_| "not logged in; run `ecoquest login`".to_string())?;
    let s: Session = serde_json::from_slice(&raw)
        .map_err(|_| "session file is invalid; run `ecoquest login`")?;
    if normalize(&s.api_url) != normalize(api_url) {
        return Err(
            "session belongs to another API URL; run `ecoquest --api-url ... login`".into(),
        );
    }
    Ok(s)
}
fn normalize(url: &str) -> String {
    url.trim_end_matches('/').to_owned()
}
fn prompt(label: &str) -> Result<String, String> {
    print!("{label}: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .map_err(|e| e.to_string())?;
    Ok(value.trim().to_owned())
}
fn confirm() -> Result<(), String> {
    if prompt("Verify participant? This awards points and may issue certificate. Type yes")?
        == "yes"
    {
        Ok(())
    } else {
        Err("cancelled".into())
    }
}
struct Api {
    client: Client,
    base: String,
    cookie: Option<String>,
}
impl Api {
    fn new(base: &str, cookie: Option<String>) -> Result<Self, String> {
        let base = normalize(base);
        if !(base.starts_with("https://")
            || base.starts_with("http://localhost")
            || base.starts_with("http://127.0.0.1"))
        {
            return Err("API URL must use HTTPS (HTTP allowed only for localhost)".into());
        }
        Ok(Self {
            client: Client::new(),
            base,
            cookie,
        })
    }
    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let mut request = self
            .client
            .request(method, format!("{}{}", self.base, path));
        if let Some(cookie) = &self.cookie {
            request = request.header(COOKIE, cookie)
        }
        if let Some(body) = body {
            request = request.json(&body)
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("API request failed: {e}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| format!("cannot read API response: {e}"))?;
        let value = serde_json::from_str(&text).unwrap_or_else(|_| json!({"message":text}));
        if !status.is_success() {
            // The API envelope is {"error":{"code":..,"message":..}}, so the
            // message is nested; the flat forms are fallbacks for proxy errors.
            let detail = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("request failed");
            return Err(format!("API {status}: {detail}"));
        }
        Ok(value)
    }
}
fn print_value(value: &Value, json_output: bool) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
        );
        return;
    }
    match value {
        Value::Array(rows) => {
            if rows.is_empty() {
                println!("No results.");
                return;
            }
            for row in rows {
                print_value(row, false)
            }
        }
        Value::Object(map) => {
            let width = map.keys().map(String::len).max().unwrap_or(0);
            for (k, v) in map {
                println!("{k:width$}  {}", cell(v), width = width)
            }
        }
        _ => println!("{}", cell(value)),
    }
}
fn cell(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        _ => value.to_string(),
    }
}
#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    if let Err(error) = run(Cli::parse()).await {
        eprintln!("error: {error}");
        std::process::exit(1)
    }
}
async fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        // Health is a readiness probe, so it must work before anyone logs in.
        Command::Health => {
            let api = Api::new(&cli.api_url, None)?;
            let value = api.call(Method::GET, "/api/health", None).await?;
            print_value(&value, cli.json);
            Ok(())
        }
        Command::Login(login) => {
            let email = match login.email {
                Some(v) => v,
                None => prompt("Email")?,
            };
            let password = match login.password {
                Some(v) => v,
                None => prompt("Password")?,
            };
            let api = Api::new(&cli.api_url, None)?;
            let response = api
                .client
                .post(format!("{}/api/auth/login", api.base))
                .json(&json!({"email":email,"password":password}))
                .send()
                .await
                .map_err(|e| format!("login failed: {e}"))?;
            let status = response.status();
            let cookies = response
                .headers()
                .get_all(SET_COOKIE)
                .iter()
                .filter_map(|h| h.to_str().ok())
                .filter_map(|v| v.split(';').next())
                .filter(|v| {
                    v.starts_with(&format!("{ACCESS_COOKIE}="))
                        || v.starts_with(&format!("{REFRESH_COOKIE}="))
                })
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let body = response.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(format!(
                    "API {status}: invalid credentials or unavailable API"
                ));
            }
            if cookies.is_empty() {
                return Err("API login did not return session cookies".into());
            }
            save_session(&Session {
                api_url: api.base,
                cookie: cookies.join("; "),
            })?;
            print_value(
                &serde_json::from_str(&body).unwrap_or(json!({"status":"logged in"})),
                cli.json,
            );
            Ok(())
        }
        command => {
            let session = load_session(&cli.api_url)?;
            let api = Api::new(&cli.api_url, Some(session.cookie))?;
            let (method, path, body, confirmation) = match command {
                Command::Me => (Method::GET, "/api/auth/me".into(), None, false),
                Command::Organizations {
                    command: OrganizationCommand::Status,
                } => (
                    Method::GET,
                    "/api/organizations/me/status".into(),
                    None,
                    false,
                ),
                Command::Events {
                    command: EventCommand::List,
                } => (Method::GET, "/api/events".into(), None, false),
                Command::Events {
                    command: EventCommand::GenerateQr { event_id },
                } => (
                    Method::POST,
                    format!("/api/events/{event_id}/qr"),
                    Some(json!({})),
                    false,
                ),
                Command::Events {
                    command: EventCommand::Create(v),
                } => (
                    Method::POST,
                    format!("/api/organizations/{}/events", v.organization_id),
                    Some(
                        json!({"name":v.name,"description":v.description,"activity_type":v.activity_type,"location":v.location,"starts_at":v.starts_at,"ends_at":v.ends_at,"capacity":v.capacity,"eco_points":v.eco_points,"impacts":[]}),
                    ),
                    false,
                ),
                Command::Participants { event_id } => (
                    Method::GET,
                    format!("/api/events/{event_id}/participants"),
                    None,
                    false,
                ),
                Command::Verify {
                    event_id,
                    participant_id,
                    yes,
                } => (
                    Method::POST,
                    format!("/api/events/{event_id}/participants/verify"),
                    Some(json!({"participation_ids":[participant_id]})),
                    !yes,
                ),
                Command::Certificates {
                    command: CertificateCommand::IssueStatus { participation_id },
                } => (
                    Method::GET,
                    format!("/api/certificates/participations/{participation_id}"),
                    None,
                    false,
                ),
                Command::Stats => (Method::GET, "/api/impact".into(), None, false),
                Command::Health | Command::Login(..) => unreachable!(),
            };
            if confirmation {
                confirm()?
            }
            let value = api.call(method, &path, body).await?;
            print_value(&value, cli.json);
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};
    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert()
    }
    #[test]
    fn parses_verify_confirmation_flag() {
        let cli =
            Cli::try_parse_from(["ecoquest", "verify", "event", "participant", "--yes"]).unwrap();
        assert!(matches!(cli.command, Command::Verify { yes: true, .. }))
    }
    #[test]
    fn rejects_nonlocal_http() {
        assert!(Api::new("http://example.test", None).is_err())
    }
    /// The API sets `eq_access` / `eq_refresh`. Login filters Set-Cookie by these
    /// names, so a rename on either side must fail here rather than silently
    /// storing an empty session.
    #[test]
    fn session_cookie_names_match_the_api() {
        assert_eq!(ACCESS_COOKIE, "eq_access");
        assert_eq!(REFRESH_COOKIE, "eq_refresh");
    }

    #[test]
    fn health_needs_no_stored_session() {
        let cli = Cli::try_parse_from(["ecoquest", "health"]).unwrap();
        assert!(matches!(cli.command, Command::Health));
    }

    #[tokio::test]
    async fn api_error_envelope_is_reported_to_the_operator() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = server.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();
            let mut request = vec![0; 1024];
            let _ = stream.read(&mut request).await.unwrap();
            let body = br#"{"error":{"code":"conflict","message":"already joined event"}}"#;
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.write_all(body).await.unwrap();
        });
        let api = Api::new(&format!("http://{address}"), None).unwrap();
        let error = api
            .call(Method::GET, "/api/events", None)
            .await
            .expect_err("expected the 409 to surface");
        assert!(
            error.contains("already joined event"),
            "operator lost the API message: {error}"
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn mock_server_request_includes_session_cookie() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = server.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();
            let mut request = vec![0; 1024];
            let size = stream.read(&mut request).await.unwrap();
            let text = String::from_utf8_lossy(&request[..size]);
            assert!(text
                .to_ascii_lowercase()
                .contains("cookie: ecoquest_access=token"));
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\n\r\n{\"ok\":true}").await.unwrap();
        });
        let api = Api::new(
            &format!("http://{address}"),
            Some("ecoquest_access=token".into()),
        )
        .unwrap();
        assert_eq!(
            api.call(Method::GET, "/api/auth/me", None).await.unwrap()["ok"],
            true
        );
        task.await.unwrap();
    }
}
