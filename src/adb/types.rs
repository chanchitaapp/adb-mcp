use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub status: String,
    pub model: Option<String>,
    pub version: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub success: bool,
}

impl CommandOutput {
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        let success = exit_code == 0;
        Self {
            stdout,
            stderr,
            exit_code,
            success,
        }
    }

    pub fn success_with_message(message: impl Into<String>) -> Self {
        Self {
            stdout: message.into(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        }
    }

    pub fn error_with_message(message: impl Into<String>) -> Self {
        Self {
            stdout: String::new(),
            stderr: message.into(),
            exit_code: 1,
            success: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogcatLine {
    pub timestamp: Option<String>,
    pub pid: Option<String>,
    pub tid: Option<String>,
    pub level: String,
    pub tag: String,
    pub message: String,
}

impl LogcatLine {
    pub fn parse(line: &str) -> Option<Self> {
        // Typical logcat format:
        // 01-15 10:00:00.123 1234 1234 I tag: message
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 6 {
            return None;
        }

        let timestamp = format!("{} {}", parts[0], parts[1]);
        let pid = parts[2].to_string();
        let tid = parts[3].to_string();
        let level = parts[4].to_string();

        // Tag and message are after "tag:"
        let tag_and_msg = parts[5..].join(" ");
        if let Some(colon_pos) = tag_and_msg.find(':') {
            let tag = tag_and_msg[..colon_pos].to_string();
            let message = tag_and_msg[colon_pos + 1..].trim().to_string();

            Some(LogcatLine {
                timestamp: Some(timestamp),
                pid: Some(pid),
                tid: Some(tid),
                level,
                tag,
                message,
            })
        } else {
            Some(LogcatLine {
                timestamp: Some(timestamp),
                pid: Some(pid),
                tid: Some(tid),
                level,
                tag: tag_and_msg,
                message: String::new(),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogcatOutput {
    pub logs: String,
    pub line_count: usize,
    pub applied_filters: Vec<String>,
}
