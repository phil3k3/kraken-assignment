# Kraken

A Rust-based CSV transaction processor.

## Prerequisites

- Rust 1.91.0 or later
- Cargo (comes with Rust)

## Building

Build the project in release mode:

```bash
cargo build --release
```

The compiled binary will be available at `target/release/kraken`.

For development builds (faster compilation, slower execution):

```bash
cargo build
```

## Running

Run the program with a CSV file as input:

```bash
cargo run -- <csv_file>
```

Or use the compiled binary:

```bash
./target/release/kraken <csv_file>
```

Example:

```bash
cargo run -- basic.csv
```

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Configuration

The application can be configured using a `Settings.toml` file in the project root. If no configuration file is present, default settings will be used.

## Project Structure

- `src/` - Source code
- `tests/` - Integration tests
- `Cargo.toml` - Project dependencies and metadata
- `Settings.toml` - Application configuration
