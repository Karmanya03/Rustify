use crate::config::{AppConfig, AuthMode, DependencyStatus, ToolStatus};
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }

    pub fn display(&self) -> String {
        let mut parts = vec![self.program.clone()];
        parts.extend(self.args.clone());
        parts.join(" ")
    }

    pub fn build_tokio(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        command
    }
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub percentage: f64,
    pub speed: String,
    pub eta: String,
}

#[derive(Debug)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
struct YtDlpAttempt {
    label: String,
    extra_args: Vec<String>,
    public_attempt: bool,
}

type ProgressLineCallback = Arc<dyn Fn(DownloadProgress) + Send + Sync>;

pub async fn dependency_status(config: &AppConfig) -> DependencyStatus {
    let yt_dlp = inspect_command(resolve_ytdlp(config), "--version").await;
    let ffmpeg = inspect_command(resolve_ffmpeg(config), "-version").await;

    DependencyStatus {
        yt_dlp,
        ffmpeg,
        auth_strategy: config.auth.describe(),
    }
}

pub fn resolve_ytdlp(config: &AppConfig) -> Option<CommandSpec> {
    if let Some(spec) = command_from_path_override(config.binaries.yt_dlp.as_deref(), Vec::new()) {
        if test_command(&spec, "--version") {
            return Some(spec);
        }
    }

    if let Some(path) = std::env::var_os("YTDLP_PATH") {
        let spec = CommandSpec::new(path.to_string_lossy().to_string(), Vec::new());
        if test_command(&spec, "--version") {
            return Some(spec);
        }
    }

    let candidates = [
        CommandSpec::new("yt-dlp", Vec::new()),
        CommandSpec::new("python", vec!["-m".to_string(), "yt_dlp".to_string()]),
        CommandSpec::new("py", vec!["-m".to_string(), "yt_dlp".to_string()]),
    ];

    candidates
        .into_iter()
        .find(|candidate| test_command(candidate, "--version"))
}

pub fn resolve_ffmpeg(config: &AppConfig) -> Option<CommandSpec> {
    if let Some(spec) = command_from_path_override(config.binaries.ffmpeg.as_deref(), Vec::new()) {
        if test_command(&spec, "-version") {
            return Some(spec);
        }
    }

    if let Some(path) = std::env::var_os("FFMPEG_PATH") {
        let spec = CommandSpec::new(path.to_string_lossy().to_string(), Vec::new());
        if test_command(&spec, "-version") {
            return Some(spec);
        }
    }

    let candidate = CommandSpec::new("ffmpeg", Vec::new());
    test_command(&candidate, "-version").then_some(candidate)
}

pub async fn run_ytdlp_capture(config: &AppConfig, args: &[String]) -> Result<CapturedOutput> {
    let spec = resolve_ytdlp(config).ok_or_else(missing_ytdlp_error)?;
    let attempts = yt_dlp_attempts(config);
    let mut last_error = None;

    for attempt in attempts {
        let mut command = spec.build_tokio();
        if disable_external_ytdlp_plugins() {
            command.arg("--no-plugin-dirs");
        }
        command.args(&attempt.extra_args);
        command.args(args);

        let output = command
            .output()
            .await
            .with_context(|| format!("Failed to execute {}", spec.display()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            return Ok(CapturedOutput { stdout, stderr });
        }

        let combined = format!("{stdout}\n{stderr}");
        last_error = Some(anyhow!(
            "yt-dlp attempt '{}' failed: {}",
            attempt.label,
            combined.trim()
        ));

        if attempt.public_attempt
            && config.auth.mode == AuthMode::Auto
            && looks_like_auth_error(&combined)
        {
            continue;
        }

        if config.auth.mode == AuthMode::Auto && !attempt.public_attempt {
            continue;
        }

        break;
    }

    Err(last_error.unwrap_or_else(missing_ytdlp_error))
}

pub async fn run_ytdlp_with_progress(
    config: &AppConfig,
    args: &[String],
    progress_callback: ProgressLineCallback,
) -> Result<CapturedOutput> {
    let spec = resolve_ytdlp(config).ok_or_else(missing_ytdlp_error)?;
    let attempts = yt_dlp_attempts(config);
    let mut last_error = None;

    for attempt in attempts {
        let mut command = spec.build_tokio();
        command
            .arg("--newline")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if disable_external_ytdlp_plugins() {
            command.arg("--no-plugin-dirs");
        }
        command.args(&attempt.extra_args);
        command.args(args);

        let (status, stdout, stderr) =
            capture_command_output(command, Some(progress_callback.clone()))
                .await
                .with_context(|| format!("Failed to execute {}", spec.display()))?;

        if status.success() {
            return Ok(CapturedOutput { stdout, stderr });
        }

        let combined = format!("{stdout}\n{stderr}");
        last_error = Some(anyhow!(
            "yt-dlp attempt '{}' failed: {}",
            attempt.label,
            combined.trim()
        ));

        if attempt.public_attempt
            && config.auth.mode == AuthMode::Auto
            && looks_like_auth_error(&combined)
        {
            continue;
        }

        if config.auth.mode == AuthMode::Auto && !attempt.public_attempt {
            continue;
        }

        break;
    }

    Err(last_error.unwrap_or_else(missing_ytdlp_error))
}

pub async fn run_ffmpeg(config: &AppConfig, args: &[String]) -> Result<()> {
    let spec = resolve_ffmpeg(config).ok_or_else(|| {
        anyhow!(
            "ffmpeg is required for conversion but was not found. Install ffmpeg or configure binaries.ffmpeg."
        )
    })?;

    let mut command = spec.build_tokio();
    command.args(args);

    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to execute {}", spec.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(anyhow!(
        "ffmpeg failed.\nstdout: {}\nstderr: {}",
        stdout.trim(),
        stderr.trim()
    ))
}

fn command_from_path_override(path: Option<&Path>, args: Vec<String>) -> Option<CommandSpec> {
    path.map(|path| CommandSpec::new(path.to_string_lossy().to_string(), args))
}

fn missing_ytdlp_error() -> anyhow::Error {
    anyhow!(
        "yt-dlp is required but was not found. Install it with `python -m pip install yt-dlp`, or set YTDLP_PATH/binaries.yt_dlp."
    )
}

fn test_command(spec: &CommandSpec, version_arg: &str) -> bool {
    let mut command = StdCommand::new(&spec.program);
    command.args(&spec.args);
    command.arg(version_arg);
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    command
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn inspect_command(spec: Option<CommandSpec>, version_arg: &str) -> ToolStatus {
    let Some(spec) = spec else {
        return ToolStatus::missing("Not installed");
    };

    let mut command = spec.build_tokio();
    command.arg(version_arg);

    match command.output().await {
        Ok(output) if output.status.success() => ToolStatus {
            available: true,
            command: Some(spec.display()),
            version: Some(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            ),
            message: None,
        },
        Ok(output) => ToolStatus {
            available: false,
            command: Some(spec.display()),
            version: None,
            message: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        },
        Err(error) => ToolStatus {
            available: false,
            command: Some(spec.display()),
            version: None,
            message: Some(error.to_string()),
        },
    }
}

fn yt_dlp_attempts(config: &AppConfig) -> Vec<YtDlpAttempt> {
    match config.auth.mode {
        AuthMode::None => vec![YtDlpAttempt {
            label: "public".to_string(),
            extra_args: Vec::new(),
            public_attempt: true,
        }],
        AuthMode::CookieFile => {
            let Some(cookie_file) = config.auth.cookie_file.as_ref() else {
                return vec![YtDlpAttempt {
                    label: "cookie-file".to_string(),
                    extra_args: Vec::new(),
                    public_attempt: false,
                }];
            };

            vec![YtDlpAttempt {
                label: "cookie-file".to_string(),
                extra_args: vec![
                    "--cookies".to_string(),
                    cookie_file.to_string_lossy().to_string(),
                ],
                public_attempt: false,
            }]
        }
        AuthMode::Browser => config
            .auth
            .browser_candidates()
            .into_iter()
            .map(|browser| YtDlpAttempt {
                label: format!("browser:{}", browser.as_yt_dlp_name()),
                extra_args: vec![
                    "--cookies-from-browser".to_string(),
                    browser.as_yt_dlp_name().to_string(),
                ],
                public_attempt: false,
            })
            .collect(),
        AuthMode::Auto => {
            let mut attempts = vec![YtDlpAttempt {
                label: "public".to_string(),
                extra_args: Vec::new(),
                public_attempt: true,
            }];

            if let Some(cookie_file) = &config.auth.cookie_file {
                attempts.push(YtDlpAttempt {
                    label: "cookie-file".to_string(),
                    extra_args: vec![
                        "--cookies".to_string(),
                        cookie_file.to_string_lossy().to_string(),
                    ],
                    public_attempt: false,
                });
            }

            attempts.extend(config.auth.browser_candidates().into_iter().map(|browser| {
                YtDlpAttempt {
                    label: format!("browser:{}", browser.as_yt_dlp_name()),
                    extra_args: vec![
                        "--cookies-from-browser".to_string(),
                        browser.as_yt_dlp_name().to_string(),
                    ],
                    public_attempt: false,
                }
            }));

            attempts
        }
    }
}

fn looks_like_auth_error(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    let markers = [
        "sign in to confirm",
        "confirm you're not a bot",
        "cookies",
        "login required",
        "members-only",
        "age-restricted",
        "authentication",
        "private video",
    ];

    markers.iter().any(|marker| lowered.contains(marker))
}

fn disable_external_ytdlp_plugins() -> bool {
    std::env::var("RUSTIFY_YTDLP_ALLOW_PLUGINS")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(true)
}

async fn capture_command_output(
    mut command: Command,
    progress_callback: Option<ProgressLineCallback>,
) -> Result<(std::process::ExitStatus, String, String)> {
    let mut child = command.spawn().context("Failed to spawn process")?;
    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let stderr = child.stderr.take().context("Failed to capture stderr")?;

    let stdout_task = tokio::spawn(read_output(stdout, None));
    let stderr_task = tokio::spawn(read_output(stderr, progress_callback));

    let status = child.wait().await.context("Failed to wait for process")?;
    let stdout = stdout_task.await.context("stdout task join failed")??;
    let stderr = stderr_task.await.context("stderr task join failed")??;

    Ok((status, stdout, stderr))
}

async fn read_output<R>(
    reader: R,
    progress_callback: Option<ProgressLineCallback>,
) -> Result<String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut output = String::new();

    while let Some(line) = lines.next_line().await? {
        if let Some(callback) = &progress_callback {
            if let Some(progress) = parse_ytdlp_progress(&line) {
                callback(progress);
            }
        }

        output.push_str(&line);
        output.push('\n');
    }

    Ok(output)
}

fn parse_ytdlp_progress(line: &str) -> Option<DownloadProgress> {
    if line.contains("[ExtractAudio]") || line.contains("[Merger]") {
        return Some(DownloadProgress {
            percentage: 99.0,
            speed: "Post-processing".to_string(),
            eta: "Finalizing".to_string(),
        });
    }

    if !line.contains("[download]") || !line.contains('%') {
        return None;
    }

    let prefix = line.split('%').next()?;
    let percentage = prefix
        .split_whitespace()
        .last()
        .and_then(|value| value.parse::<f64>().ok())?;

    let speed = line
        .split(" at ")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .unwrap_or("Working")
        .to_string();

    let eta = line
        .split(" ETA ")
        .nth(1)
        .and_then(|value| value.split_whitespace().next())
        .unwrap_or("Unknown")
        .to_string();

    Some(DownloadProgress {
        percentage,
        speed,
        eta,
    })
}
