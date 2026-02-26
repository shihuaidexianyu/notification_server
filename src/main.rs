use std::{env, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

#[derive(Debug)]
struct Config {
    http_bind: String,
    smtp_host: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    smtp_from: Mailbox,
    smtp_tls: bool,
}

#[derive(Debug, Deserialize)]
struct SendEmailRequest {
    title: String,
    to: String,
    body: String,
}

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cfg = Config::from_env().context("failed to load configuration from environment")?;
    let mailer = build_mailer(&cfg)?;

    let state = Arc::new(AppState {
        mailer,
        from: cfg.smtp_from,
    });

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/send-email", post(send_email))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.http_bind)
        .await
        .with_context(|| format!("failed to bind to {}", cfg.http_bind))?;

    info!(addr = %cfg.http_bind, "server started");
    axum::serve(listener, app)
        .await
        .context("http server exited unexpectedly")
}

async fn healthz() -> impl IntoResponse {
    Json(ApiResponse {
        ok: true,
        message: "ok".to_string(),
    })
}

async fn send_email(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendEmailRequest>,
) -> impl IntoResponse {
    if req.title.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "title cannot be empty");
    }
    if req.body.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "body cannot be empty");
    }
    if req.to.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "to cannot be empty");
    }

    let to = match Mailbox::from_str(req.to.trim()) {
        Ok(mailbox) => mailbox,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid recipient email"),
    };

    let email = match Message::builder()
        .from(state.from.clone())
        .to(to.clone())
        .subject(req.title)
        .body(req.body)
    {
        Ok(message) => message,
        Err(err) => {
            error!(error = %err, "failed to build message");
            return error_response(StatusCode::BAD_REQUEST, "invalid email payload");
        }
    };

    match state.mailer.send(email).await {
        Ok(_) => {
            info!(to = %to, "email sent");
            (
                StatusCode::OK,
                Json(ApiResponse {
                    ok: true,
                    message: "sent".to_string(),
                }),
            )
        }
        Err(err) => {
            error!(to = %to, error = %err, "smtp send failed");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "smtp send failed")
        }
    }
}

fn build_mailer(cfg: &Config) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
    let credentials = Credentials::new(cfg.smtp_username.clone(), cfg.smtp_password.clone());

    let mailer = if cfg.smtp_tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
            .context("failed to create TLS SMTP transport")?
            .port(cfg.smtp_port)
            .credentials(credentials)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
            .port(cfg.smtp_port)
            .credentials(credentials)
            .build()
    };

    Ok(mailer)
}

impl Config {
    fn from_env() -> Result<Self> {
        let smtp_from_raw = must_env("SMTP_FROM")?;
        let smtp_from =
            Mailbox::from_str(&smtp_from_raw).context("SMTP_FROM is not a valid email")?;

        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse::<u16>()
            .context("SMTP_PORT must be a valid u16")?;

        Ok(Self {
            http_bind: env::var("HTTP_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            smtp_host: must_env("SMTP_HOST")?,
            smtp_port,
            smtp_username: must_env("SMTP_USERNAME")?,
            smtp_password: must_env("SMTP_PASSWORD")?,
            smtp_from,
            smtp_tls: parse_bool_env("SMTP_TLS").unwrap_or(true),
        })
    }
}

fn must_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing env var: {name}"))
}

fn parse_bool_env(name: &str) -> Option<bool> {
    let raw = env::var(name).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<ApiResponse>) {
    (
        status,
        Json(ApiResponse {
            ok: false,
            message: message.to_string(),
        }),
    )
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "stmp_server=info,axum=info".into());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
