# Ghostkey CLI

A command-line interface for managing [ghost keys](https://freenet.org/ghostkey/) and certificates
in the [Freenet](https://freenet.org/) ecosystem.

## What are Ghost Keys?

Ghost keys are a cryptographic mechanism used in the Freenet ecosystem to provide anonymous,
unlinkable donations. They allow donors to prove they have made a donation without revealing their
identity or linking multiple donations together. Ghost keys are created through a multi-step process
involving master keys, delegate certificates, and finally the ghost key itself.

## Purpose of Ghost Keys

1. **Anonymity**: Donors can prove they've made a donation without revealing their identity.
2. **Verifiability**: The system can verify that a donation has been made without knowing who made
   it.

This CLI tool provides the necessary utilities to manage the entire lifecycle of ghost keys, from
generating master keys to creating and verifying ghost key certificates.

## Features

- Generate master keys
- Create and verify delegate certificates
- Generate and verify ghost key certificates
- Sign messages with ghost keys
- Verify signed messages

## Installation

To install the Ghostkey CLI, you need to have Rust and Cargo installed on your system:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Then, you can build and install the CLI using:

```bash
cargo install ghostkey
```

## Usage

```bash
$ ghostkey -h
Usage: ghostkey [COMMAND]

Commands:
  generate-master-key  Generate a new master keypair
  generate-delegate    Generates a new delegate signing key and certificate
  verify-delegate      Verifies a delegate key certificate using the master verifying key
  generate-ghost-key   Generates a ghost key from a delegate signing key
  verify-ghost-key     Verifies a ghost key certificate using the master verifying key
  help                 Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

ghostkey <subcommand> --help
```

## Examples

1. Verify a ghost key certificate:

   ```
   ghostkey verify-ghost-key --ghost-certificate ./ghost-key/ghost_key_certificate.pem
   ```

2. Sign a message:

   ```
   ghostkey sign-message --ghost-certificate ./ghost-key/ghost_key_certificate.pem --ghost-signing-key ./ghost-key/ghost_key_signing_key.pem --message ./message.txt --output ./signed_message.pem
   ```

3. Verify a signed message:
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

This project is licensed under the
[GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.html).
