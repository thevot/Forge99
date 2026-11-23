# Security Policy

## Local & Offline Design

Forge99 runs entirely offline on your machine. It does not open network listeners, transmit telemetry, or bundle proprietary game binaries.

When parsing untrusted Unreal Engine 1 package files (`.unr`, `.utx`, `.uax`, `.umx`, `.u`) or `.t3d` text exports, binary parsing is implemented in memory-safe Rust. If you discover a parser panic, out-of-memory condition, path-traversal issue during package extraction, or any other security concern, please report it responsibly.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for security vulnerabilities.

Instead, report vulnerabilities privately through **GitHub Security Advisories** on the `thevot/Forge99` repository (`Security` tab -> `Report a vulnerability`).

Include as much of the following as possible:
- A description of the issue and potential impact
- Steps or a minimal test file to reproduce the behavior
- Your Forge99 version and operating system

We will acknowledge reports as promptly as possible and coordinate a fix before public disclosure.
