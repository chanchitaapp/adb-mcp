# ADB MCP

A high-performance Rust implementation of the Model Context Protocol (MCP) for Android Debug Bridge (ADB).

## Usage

Add the following to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "adb": {
      "command": "/absolute/path/to/adb-mcp"
    }
  }
}
```

## Available Tools

- **Device**: `adb_devices`
- **Shell**: `adb_shell`, `adb_activity_manager`, `adb_package_manager`
- **Apps**: `adb_install`, `adb_pull`, `adb_push`
- **UI**: `dump_image`, `inspect_ui`
- **Logs**: `adb_logcat` (with advanced filtering)

## License

MIT
