# Security policy

TermZip launches a shell with the permissions of the current user. Treat the application, its configuration, and any
future extensions as part of your local execution environment.

## Supported versions

Before the first stable release, security fixes are developed against the latest `main` branch. Once releases begin,
the latest release receives security fixes while the project is below 1.0.

## Reporting a vulnerability

Please do not report exploitable vulnerabilities in public issues. Use
[GitHub's private security advisory form](https://github.com/LuisPeixoto/TermZip/security/advisories/new). Include
the affected version, operating system, impact, and a reproduction or proof of concept when safe to do so.

## Security design

- The shell inherits the current user's normal operating-system permissions.
- TermZip does not send telemetry or terminal contents to a remote service.
- PTY output is kept in memory; the initial release does not persist session transcripts.
- Plugin loading is not implemented. Future extensions require an explicit isolation and permission design.
- Terminal escape sequences and shell output must be treated as untrusted input.
