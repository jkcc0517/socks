# ⚠ Resource Management Warning
This project has known limitations in resource management and error handling.
Specifically, connection errors and timeouts are not properly handled, which may lead to unexpected behavior or resource exhaustion.
It is not recommended for use in production environments. Please use with caution and monitor execution closely.

# SOCKS5 Proxy Server

A lightweight and fast SOCKS5 proxy server implementation in Rust.

## Features

- Only SOCKS5 protocol support
- TCP connection support
- UDP associate support(Testing)
- IPv4 and IPv6(Testing) support
- ~~Domain name resolution~~
- Command-line interface

## Installation

### Prerequisites

- Rust 1.70.0 or higher
- Cargo (Rust's package manager)

### Building from source

```bash
# Clone the repository
git clone https://github.com/yourusername/socks
cd socks

# Build the project
cargo build --release

# The binary will be available at target/release/socks
```

## Usage

You can run the SOCKS5 proxy server using the following command:

```bash
cargo run -- [OPTIONS]
```

Or if you're using the compiled binary:

```bash
./target/release/socks [OPTIONS]
```

### Command Line Options

| Option | Description | Default Value |
|--------|-------------|---------------|
| `--host` | Host address to bind | 127.0.0.1 |
| `--port` | Port number to listen on | 1080 |
| `-v, --verbose` | Enable verbose logging | false |
| `--help` | Display help information | - |
| `--version` | Display version information | - |

### Examples

1. Run with default settings (localhost:1080):
```bash
cargo run
```

2. Bind to a specific IP and port:
```bash
cargo run -- --host 0.0.0.0 --port 1081
```

3. Enable verbose logging:
```bash
cargo run -- --verbose
```

4. Show help information:
```bash
cargo run -- --help
```

## Environment Variables

The following environment variables can be used to configure the server:

- `RUST_LOG`: Set logging level (error, warn, info, debug, trace)
  - This will be overridden by the `--verbose` flag if specified

## Client Configuration

### Using with curl

```bash
curl --socks5 127.0.0.1:1080 https://example.com
```

### Using with Firefox

1. Open Firefox Settings
2. Search for "proxy"
3. Click "Settings..." in the Network Settings section
4. Select "Manual proxy configuration"
5. Enter SOCKS Host: 127.0.0.1
6. Enter Port: 1080
7. Select SOCKS v5
8. Click "OK"

## Development

### Running Tests

```bash
cargo test
```

### Running with Different Log Levels

```bash
# Error level only
RUST_LOG=error cargo run

# Debug level
RUST_LOG=debug cargo run

# Or use the verbose flag
cargo run -- --verbose
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Troubleshooting

### Common Issues

1. Address already in use
```bash
# Change the port number
cargo run -- --port 1081
```

2. Permission denied when binding to port < 1024
```bash
# Run with sudo (Linux/macOS)
sudo cargo run -- --port 443
```

3. Connection issues
- Check if the server is running
- Verify the host and port settings
- Check your firewall settings
- Enable verbose logging for more detailed information
```bash
cargo run -- --verbose
```

## Security Considerations

- The server currently doesn't implement authentication