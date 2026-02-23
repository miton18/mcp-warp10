# MCP Warp10 Server

A Model Context Protocol (MCP) server for interacting with Warp10 time series database.

## Features

- **Fetch**: Query time series data with flexible filtering
- **Update**: Insert or update data points
- **Delete**: Remove time series data
- **Execute**: Run WarpScript directly
- **Find**: Discover available time series
- **Meta**: Manage time series metadata
- **Label Values**: Discover unique values for any label
- **Predict**: Advanced forecasting with multiple algorithms (Holt-Winters, ARIMA, Linear Regression, FFT, and more)

## Installation

```bash
npm install @mcp/warp10
```

Or install from source:

```bash
git clone https://github.com/yourusername/mcp-warp10
cd mcp-warp10
npm install
npm run build
```

## Configuration

### Claude Desktop

Add to your Claude Desktop configuration (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "warp10": {
      "command": "node",
      "args": ["/path/to/mcp-warp10/dist/index.js"],
      "env": {
        "WARP10_ENDPOINT": "http://localhost:8080/api/v0",
        "WARP10_TOKEN": "your-read-write-token"
      }
    }
  }
}
```

### Alternative Token Configuration

You can use separate tokens for read and write operations:

```json
{
  "mcpServers": {
    "warp10": {
      "command": "node",
      "args": ["/path/to/mcp-warp10/dist/index.js"],
      "env": {
        "WARP10_ENDPOINT": "http://localhost:8080/api/v0",
        "WARP10_READ_TOKEN": "your-read-token",
        "WARP10_WRITE_TOKEN": "your-write-token"
      }
    }
  }
}
```

## Usage Examples

### Fetching Data

```typescript
// Fetch last hour of temperature data
{
  "tool": "warp10_fetch",
  "arguments": {
    "selector": "sensor.temperature",
    "labels": {
      "location": "room1"
    },
    "start": "-1h",
    "end": "NOW"
  }
}
```

### Inserting Data

```typescript
// Insert temperature readings
{
  "tool": "warp10_update",
  "arguments": {
    "className": "sensor.temperature",
    "labels": {
      "location": "room1",
      "sensor": "DHT22"
    },
    "dataPoints": [
      {
        "timestamp": 1708500000000000,
        "value": 22.5
      },
      {
        "timestamp": 1708500060000000,
        "value": 22.7
      }
    ]
  }
}
```

### Executing WarpScript

```typescript
// Execute custom WarpScript
{
  "tool": "warp10_exec",
  "arguments": {
    "script": "[ 'token' 'sensor.*' {} NOW -1 h ] FETCH"
  }
}
```

### Finding Time Series

```typescript
// Find all temperature sensors
{
  "tool": "warp10_find",
  "arguments": {
    "selector": "~sensor.temp.*",
    "labels": {
      "active": "true"
    }
  }
}
```

### Discovering Label Values

```typescript
// Get all unique locations
{
  "tool": "warp10_label_values",
  "arguments": {
    "selector": "~sensor.*",
    "labelName": "location"
  }
}
```

### Predicting Future Values

```typescript
// Predict next 24 hours using Holt-Winters
{
  "tool": "warp10_predict",
  "arguments": {
    "selector": "power.consumption",
    "method": "holt_winters",
    "historyPeriod": "-30d",
    "predictionHorizon": 24,
    "bucketSpan": "1h"
  }
}
```

## WarpScript Support

The server includes full WarpScript execution support through the `warp10_exec` tool. You can run any valid WarpScript code, including:

- Data fetching and manipulation
- Mathematical operations
- Control flow (loops, conditions)
- Custom functions and macros
- Geo-temporal operations

## Resources

The server provides two built-in resources:

1. **Documentation** (`warp10://documentation`): Complete guide to using the MCP server
2. **WarpScript Reference** (`warp10://warpscript-reference`): Quick reference for WarpScript language

## Environment Variables

- `WARP10_ENDPOINT`: Warp10 server endpoint (default: `http://localhost:8080/api/v0`)
- `WARP10_TOKEN`: Read/write token for Warp10
- `WARP10_READ_TOKEN`: Read-only token (optional, overrides WARP10_TOKEN for reads)
- `WARP10_WRITE_TOKEN`: Write-only token (optional, overrides WARP10_TOKEN for writes)

## Development

```bash
# Install dependencies
npm install

# Build the project
npm run build

# Run in development mode
npm run dev
```

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.