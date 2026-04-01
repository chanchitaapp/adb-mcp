use adb_mcp::adb::AdbExecutor;
use adb_mcp::filters::logcat::LogLevel;
use adb_mcp::filters::LogcatFilterChain;
use adb_mcp::mcp::server::InputSchema;
use adb_mcp::mcp::McpServer;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use log::error;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .target(env_logger::Target::Stderr)
        .try_init()?;

    // Create ADB executor
    let adb_path = std::env::var("ADB_PATH").ok();
    let executor = Arc::new(AdbExecutor::new(adb_path));

    // Create MCP server
    let mut mcp_server = McpServer::new("adb-mcp", "0.1.0");

    // ===== adb_devices tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_devices",
            "Lists all connected Android devices and emulators with their status, model, and version.",
            InputSchema::new().to_json(),
            move |_args| {
                let executor = Arc::clone(&executor);
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.list_devices().await })
                }) {
                    Ok(devices) => {
                        let device_list: Vec<_> = devices.iter().map(|d| {
                            json!({
                                "id": d.id,
                                "status": d.status,
                                "model": d.model,
                                "version": d.version,
                            })
                        }).collect();
                        Ok(json!({"devices": device_list}))
                    }
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_shell tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_shell",
            "Executes a shell command on a connected Android device.",
            InputSchema::new()
                .add_property("command", "string", "Shell command to execute", true)
                .add_property("device", "string", "Target device ID (optional)", false)
                .to_json(),
            move |args| {
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'command' parameter")?;
                let device = args.get("device").and_then(|v| v.as_str());

                let executor = Arc::clone(&executor);
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.shell(device, command).await })
                }) {
                    Ok(output) => Ok(json!({
                        "stdout": output.stdout,
                        "stderr": output.stderr,
                        "exit_code": output.exit_code,
                        "success": output.success,
                    })),
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_logcat tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_logcat",
            "Retrieves Android system logs with advanced filtering. Supports keywords, regex, log levels, tags, and exclusions.",
            InputSchema::new()
                .add_property("device", "string", "Target device ID (optional)", false)
                .add_property("filter", "string", "Logcat filter expression (optional)", false)
                .add_property("keywords", "array", "Keyword filters (case-insensitive, optional)", false)
                .add_property("min_level", "string", "Minimum log level: V/D/I/W/E/F (optional)", false)
                .add_property("tags", "array", "Specific log tags to filter by (optional)", false)
                .add_property("exclude", "array", "Patterns to exclude from results (optional)", false)
                .add_property("regex", "string", "Regex pattern for advanced filtering (optional)", false)
                .add_property("lines", "number", "Maximum lines to return (default: 50)", false)
                .to_json(),
            move |args| {
                let device = args.get("device").and_then(|v| v.as_str());
                let filter = args.get("filter").and_then(|v| v.as_str());
                let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(50) as u32;

                let executor = Arc::clone(&executor);

                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.logcat(device, filter, Some(lines)).await })
                }) {
                    Ok(output) => {
                        let mut filter_chain = LogcatFilterChain::new();
                        let mut applied_filters = vec![];

                        // Add keyword filter
                        if let Some(keywords_val) = args.get("keywords") {
                            if let Some(keywords_array) = keywords_val.as_array() {
                                let keywords: Vec<String> = keywords_array
                                    .iter()
                                    .filter_map(|k| k.as_str().map(|s| s.to_string()))
                                    .collect();
                                if !keywords.is_empty() {
                                    filter_chain = filter_chain.add_keyword_filter(keywords, false);
                                    applied_filters.push("keyword".to_string());
                                }
                            }
                        }

                        // Add log level filter
                        if let Some(level_str) = args.get("min_level").and_then(|v| v.as_str()) {
                            if let Some(level) = LogLevel::from_str(level_str) {
                                filter_chain = filter_chain.add_level_filter(level);
                                applied_filters.push(format!("min_level:{}", level_str));
                            }
                        }

                        // Add tag filter
                        if let Some(tags_val) = args.get("tags") {
                            if let Some(tags_array) = tags_val.as_array() {
                                let tags: Vec<String> = tags_array
                                    .iter()
                                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                                    .collect();
                                if !tags.is_empty() {
                                    filter_chain = filter_chain.add_tag_filter(tags);
                                    applied_filters.push("tags".to_string());
                                }
                            }
                        }

                        // Add exclude filter
                        if let Some(exclude_val) = args.get("exclude") {
                            if let Some(exclude_array) = exclude_val.as_array() {
                                let patterns: Vec<String> = exclude_array
                                    .iter()
                                    .filter_map(|p| p.as_str().map(|s| s.to_string()))
                                    .collect();
                                if !patterns.is_empty() {
                                    filter_chain = filter_chain.add_exclude_filter(patterns);
                                    applied_filters.push("exclude".to_string());
                                }
                            }
                        }

                        // Apply base filters first
                        let mut filtered_output = filter_chain.apply(&output.stdout);

                        // Apply regex filter separately (if provided)
                        if let Some(regex_str) = args.get("regex").and_then(|v| v.as_str()) {
                            match adb_mcp::filters::logcat::RegexFilter::new(regex_str) {
                                Ok(regex_filter) => {
                                    // Manually apply regex filter
                                    filtered_output = filtered_output
                                        .lines()
                                        .filter(|line| {
                                            use adb_mcp::filters::logcat::LogcatFilter;
                                            regex_filter.matches(line)
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    applied_filters.push(format!("regex:{}", regex_str));
                                }
                                Err(_) => {
                                    eprintln!("[WARN] Invalid regex pattern: {}", regex_str);
                                }
                            }
                        }

                        let line_count = filtered_output.lines().count();

                        Ok(json!({
                            "logs": filtered_output,
                            "line_count": line_count,
                            "applied_filters": applied_filters,
                        }))
                    }
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_install tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_install",
            "Installs an Android application (APK) on a connected device.",
            InputSchema::new()
                .add_property("apk_path", "string", "Local path to the APK file", true)
                .add_property("device", "string", "Target device ID (optional)", false)
                .to_json(),
            move |args| {
                let apk_path = args
                    .get("apk_path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'apk_path' parameter")?;
                let device = args.get("device").and_then(|v| v.as_str());

                let executor = Arc::clone(&executor);
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.install(device, apk_path).await })
                }) {
                    Ok(output) => Ok(json!({
                        "success": output.success,
                        "stdout": output.stdout,
                        "stderr": output.stderr,
                    })),
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_pull tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_pull",
            "Transfers a file from a connected Android device to the local system.",
            InputSchema::new()
                .add_property("remote_path", "string", "Remote file path on device", true)
                .add_property("device", "string", "Target device ID (optional)", false)
                .add_property(
                    "as_base64",
                    "boolean",
                    "Return file content as base64 (default: false)",
                    false,
                )
                .to_json(),
            move |args| {
                let remote_path = args
                    .get("remote_path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'remote_path' parameter")?;
                let device = args.get("device").and_then(|v| v.as_str());
                let as_base64 = args
                    .get("as_base64")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let temp_file = format!("/tmp/adb-mcp-pull-{}", uuid::Uuid::new_v4());
                let executor = Arc::clone(&executor);

                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.pull(device, remote_path, &temp_file).await })
                }) {
                    Ok(_) => {
                        // Read the file
                        match std::fs::read(&temp_file) {
                            Ok(content) => {
                                let result_str = if as_base64 {
                                    STANDARD.encode(&content)
                                } else {
                                    String::from_utf8_lossy(&content).to_string()
                                };
                                let _ = std::fs::remove_file(&temp_file);
                                Ok(json!({
                                    "success": true,
                                    "data": result_str,
                                    "path": remote_path,
                                }))
                            }
                            Err(e) => {
                                let _ = std::fs::remove_file(&temp_file);
                                Err(format!("Failed to read pulled file: {}", e))
                            }
                        }
                    }
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_activity_manager tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_activity_manager",
            "Executes Activity Manager (am) commands on a device (start, broadcast, force-stop, etc.)",
            InputSchema::new()
                .add_property("subcommand", "string", "Activity Manager subcommand (e.g., 'start', 'broadcast')", true)
                .add_property("args", "string", "Arguments for the subcommand (optional)", false)
                .add_property("device", "string", "Target device ID (optional)", false)
                .to_json(),
            move |args| {
                let subcommand = args.get("subcommand")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'subcommand' parameter")?;
                let am_args = args.get("args").and_then(|v| v.as_str());
                let device = args.get("device").and_then(|v| v.as_str());

                let executor = Arc::clone(&executor);
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.am(device, subcommand, am_args).await })
                }) {
                    Ok(output) => {
                        Ok(json!({
                            "success": output.success,
                            "stdout": output.stdout,
                            "stderr": output.stderr,
                        }))
                    }
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    // ===== adb_package_manager tool =====
    {
        let executor = Arc::clone(&executor);
        mcp_server = mcp_server.register_tool(
            "adb_package_manager",
            "Executes Package Manager (pm) commands on a device (list packages, grant permissions, etc.)",
            InputSchema::new()
                .add_property("subcommand", "string", "Package Manager subcommand (e.g., 'list', 'grant')", true)
                .add_property("args", "string", "Arguments for the subcommand (optional)", false)
                .add_property("device", "string", "Target device ID (optional)", false)
                .to_json(),
            move |args| {
                let subcommand = args.get("subcommand")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'subcommand' parameter")?;
                let pm_args = args.get("args").and_then(|v| v.as_str());
                let device = args.get("device").and_then(|v| v.as_str());

                let executor = Arc::clone(&executor);
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async { executor.pm(device, subcommand, pm_args).await })
                }) {
                    Ok(output) => {
                        Ok(json!({
                            "success": output.success,
                            "stdout": output.stdout,
                            "stderr": output.stderr,
                        }))
                    }
                    Err(e) => Err(e.to_string()),
                }
            },
        );
    }

    eprintln!("[INFO] Starting adb-mcp v0.1.0");
    eprintln!("[INFO] ADB MCP Server connected and ready");
    eprintln!("[INFO] Registered 7 tools");

    // Set up stdio transport
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    // Main message loop
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                eprintln!("[INFO] Received EOF, shutting down");
                break;
            }
            Ok(_) => {
                let line_trimmed = line.trim();
                if !line_trimmed.is_empty() {
                    let response = mcp_server.handle_message(line_trimmed.to_string()).await;
                    if let Err(e) = stdout.write_all(response.as_bytes()).await {
                        error!("Failed to write response: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.write_all(&b"\n"[..]).await {
                        error!("Failed to write newline: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.flush().await {
                        error!("Failed to flush stdout: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}
