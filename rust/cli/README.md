# Ghostkey CLI

A command-line interface for managing ghost keys and certificates in the Freenet ecosystem.

## Features

- Generate master keys
- Create and verify delegate certificates
- Generate and verify ghost key certificates
- Sign messages with ghost keys
- Verify signed messages

## Installation

To install the Ghostkey CLI, you need to have Rust and Cargo installed on your system. Then, you can build and install the CLI using:

```
cargo install --path .
```

## Usage

The Ghostkey CLI provides several subcommands:

- `generate-master-key`: Generate a new master key pair
- `generate-delegate`: Create a new delegate certificate
- `verify-delegate`: Verify a delegate certificate
- `generate-ghost-key`: Generate a new ghost key certificate
- `verify-ghost-key`: Verify a ghost key certificate
- `sign-message`: Sign a message using a ghost key
- `verify-signed-message`: Verify a signed message

For detailed usage of each subcommand, use the `--help` flag:

```
ghostkey <subcommand> --help
```

## Examples

1. Generate a master key:
   ```
   ghostkey generate-master-key --output-dir ./master-keys
   ```

2. Create a delegate certificate:
   ```
   ghostkey generate-delegate --master-signing-key ./master-keys/master_signing_key.pem --info "Test Delegate" --output-dir ./delegate
   ```

3. Generate a ghost key:
   ```
   ghostkey generate-ghost-key --delegate-dir ./delegate --output-dir ./ghost-key
   ```

4. Sign a message:
   ```
   ghostkey sign-message --ghost-certificate ./ghost-key/ghost_key_certificate.pem --ghost-signing-key ./ghost-key/ghost_key_signing_key.pem --message ./message.txt --output ./signed_message.pem
   ```

5. Verify a signed message:
   ```
   ghostkey verify-signed-message --signed-message ./signed_message.pem --master-verifying-key ./master-keys/master_verifying_key.pem
   ```

## Testing

To run the test suite for the Ghostkey CLI, use:

```
./test_ghostkey.sh
```

This script will run through various scenarios to ensure the CLI is functioning correctly.

## License

This project is licensed under the [GNU Lesser General Public License v3.0](https://www.gnu.org/licenses/lgpl-3.0.html).
