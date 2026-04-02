use crate::adb::types::{CommandOutput, Device};
use crate::errors::{AdbError, Result};
use std::process::Stdio;
use tokio::process::Command;
use log::{debug, warn};

#[derive(Clone)]
pub struct AdbExecutor {
    adb_path: String,
}

impl AdbExecutor {
    pub fn new(adb_path: Option<String>) -> Self {
        let path = adb_path.unwrap_or_else(|| "adb".to_string());
        Self { adb_path: path }
    }

    /// Execute an ADB command with the given arguments
    pub async fn execute(&self, args: Vec<&str>) -> Result<CommandOutput> {
        debug!("Executing ADB command: {} {:?}", self.adb_path, args);

        let mut cmd = Command::new(&self.adb_path);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| AdbError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if !output.status.success() && !stderr.is_empty() {
            warn!("ADB command failed: {}", stderr);
        }

        Ok(CommandOutput::new(stdout, stderr, exit_code))
    }

    /// List all connected devices
    pub async fn list_devices(&self) -> Result<Vec<Device>> {
        let output = self.execute(vec!["devices", "-l"]).await?;

        let mut devices = Vec::new();
        for line in output.stdout.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let device_id = parts[0].to_string();
            let status = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

            // Parse model and version from remaining parts
            let mut model = None;
            let mut version = None;
            let mut device_type = None;

            for part in parts.iter().skip(2) {
                if part.starts_with("model:") {
                    model = Some(part.strip_prefix("model:").unwrap_or("").to_string());
                } else if part.starts_with("device:") {
                    device_type = Some(part.strip_prefix("device:").unwrap_or("").to_string());
                } else if part.starts_with("version:") {
                    version = Some(part.strip_prefix("version:").unwrap_or("").to_string());
                }
            }

            devices.push(Device {
                id: device_id,
                status,
                model,
                version,
                device_type,
            });
        }

        Ok(devices)
    }

    /// Execute a shell command on a specific device
    pub async fn shell(&self, device: Option<&str>, command: &str) -> Result<CommandOutput> {
        let mut args = vec![];

        if let Some(d) = device {
            args.push("-s");
            args.push(d);
        }

        args.push("shell");

        // Parse the shell command into individual arguments
        let shell_args: Vec<&str> = command.split_whitespace().collect();
        args.extend(shell_args);

        self.execute(args).await
    }

    /// Get logcat output
    pub async fn logcat(
        &self,
        device: Option<&str>,
        filter: Option<&str>,
        lines: Option<u32>,
    ) -> Result<CommandOutput> {
        let mut args = vec![];

        if let Some(d) = device {
            args.push("-s");
            args.push(d);
        }

        args.push("logcat");
        args.push("-d"); // Dump logcat and exit

        // Add filter if provided (split into separate arguments for logcat syntax)
        if let Some(f) = filter {
            // For filters like "TAG:LEVEL *:S", we need to split them as separate arguments
            // but preserve the format expected by adb logcat
            let filter_parts: Vec<&str> = f.split_whitespace().collect();
            args.extend(filter_parts);
        }

        let mut output = self.execute(args).await?;

        // Limit lines if specified
        if let Some(limit) = lines {
            let lines_vec: Vec<&str> = output.stdout.lines().collect();
            let start = if lines_vec.len() > limit as usize {
                lines_vec.len() - limit as usize
            } else {
                0
            };

            output.stdout = lines_vec[start..].join("\n");
        }

        Ok(output)
    }

    /// Pull a file from device to local path
    pub async fn pull(&self, device: Option<&str>, remote: &str, local: &str) -> Result<CommandOutput> {
        let mut args = vec![];

        if let Some(d) = device {
            args.push("-s");
            args.push(d);
        }

        args.push("pull");
        args.push(remote);
        args.push(local);

        self.execute(args).await
    }

    /// Push a file from local path to device
    pub async fn push(&self, device: Option<&str>, local: &str, remote: &str) -> Result<CommandOutput> {
        let mut args = vec![];

        if let Some(d) = device {
            args.push("-s");
            args.push(d);
        }

        args.push("push");
        args.push(local);
        args.push(remote);

        self.execute(args).await
    }

    /// Install an APK
    pub async fn install(&self, device: Option<&str>, apk_path: &str) -> Result<CommandOutput> {
        let mut args = vec![];

        if let Some(d) = device {
            args.push("-s");
            args.push(d);
        }

        args.push("install");
        args.push("-r");
        args.push(apk_path);

        self.execute(args).await
    }

    /// Execute Activity Manager command
    pub async fn am(
        &self,
        device: Option<&str>,
        subcommand: &str,
        args: Option<&str>,
    ) -> Result<CommandOutput> {
        let mut cmd_args = vec![];

        if let Some(d) = device {
            cmd_args.push("-s");
            cmd_args.push(d);
        }

        cmd_args.push("shell");
        cmd_args.push("am");
        cmd_args.push(subcommand);

        if let Some(a) = args {
            let additional_args: Vec<&str> = a.split_whitespace().collect();
            cmd_args.extend(additional_args);
        }

        self.execute(cmd_args).await
    }

    /// Execute Package Manager command
    pub async fn pm(
        &self,
        device: Option<&str>,
        subcommand: &str,
        args: Option<&str>,
    ) -> Result<CommandOutput> {
        let mut cmd_args = vec![];

        if let Some(d) = device {
            cmd_args.push("-s");
            cmd_args.push(d);
        }

        cmd_args.push("shell");
        cmd_args.push("pm");
        cmd_args.push(subcommand);

        if let Some(a) = args {
            let additional_args: Vec<&str> = a.split_whitespace().collect();
            cmd_args.extend(additional_args);
        }

        self.execute(cmd_args).await
    }

    /// Take a screenshot
    pub async fn screenshot(&self, device: Option<&str>) -> Result<CommandOutput> {
        let temp_file = format!("/sdcard/screenshot-{}.png", uuid::Uuid::new_v4());

        // Take screenshot on device
        self.shell(device, &format!("screencap -p {}", temp_file))
            .await?;

        // Create a temporary local file path
        let local_file = std::env::temp_dir().join(format!("screenshot-{}.png", uuid::Uuid::new_v4()));
        let local_path = local_file.to_string_lossy();

        // Pull the screenshot
        self.pull(device, &temp_file, &local_path).await?;

        // Clean up remote file
        self.shell(device, &format!("rm {}", temp_file)).await?;

        Ok(CommandOutput::success_with_message(format!(
            "Screenshot saved to {}",
            local_path
        )))
    }

    /// Dump UI hierarchy
    pub async fn dump_ui(&self, device: Option<&str>) -> Result<CommandOutput> {
        let remote_path = format!("/sdcard/window_dump-{}.xml", uuid::Uuid::new_v4());

        // Dump UI on device
        self.shell(device, &format!("uiautomator dump {}", remote_path))
            .await?;

        // Create a temporary local file path
        let local_file = std::env::temp_dir().join(format!("window_dump-{}.xml", uuid::Uuid::new_v4()));
        let local_path = local_file.to_string_lossy();

        // Pull the UI dump
        let result = self.pull(device, &remote_path, &local_path).await?;

        // Clean up remote file
        self.shell(device, &format!("rm {}", remote_path)).await.ok();

        if result.success {
            // Read and return the XML content
            let xml_content = tokio::fs::read_to_string(local_path.as_ref()).await?;
            // Clean up local file
            tokio::fs::remove_file(local_path.as_ref()).await.ok();
            Ok(CommandOutput::success_with_message(xml_content))
        } else {
            Ok(result)
        }
    }
}
