use crate::adb::AdbExecutor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AdbServer {
    pub executor: Arc<AdbExecutor>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ListDevicesOutput {
    pub devices: Vec<DeviceInfo>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DeviceInfo {
    pub id: String,
    pub status: String,
    pub model: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogcatOutput {
    pub logs: String,
    pub line_count: usize,
    pub applied_filters: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScreenshotOutput {
    pub path: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UiDumpOutput {
    pub xml: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstallOutput {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileTransferOutput {
    pub success: bool,
    pub message: String,
    pub data: Option<String>,
}

impl AdbServer {
    pub fn new(executor: Arc<AdbExecutor>) -> Self {
        Self { executor }
    }

    pub fn get_executor(&self) -> Arc<AdbExecutor> {
        Arc::clone(&self.executor)
    }
}
