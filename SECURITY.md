# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in aicite, please report it responsibly.

**Do not open a public issue.**

Instead, send an email to **[risaavedraf]** with:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact assessment
- Any suggested fix (if available)

## Response Timeline

| Step | Expected Timeframe |
|---|---|
| Acknowledgment of report | Within 48 hours |
| Initial assessment | Within 5 business days |
| Fix or mitigation | Depends on severity |
| Public disclosure | After fix is released |

We will keep you informed of progress and coordinate disclosure timing with you.

## Scope

The following areas are considered in-scope for security reports:

### API Key Handling

- Storage and transmission of API keys for LLM providers
- Exposure of keys in logs, error messages, or debug output
- Key leakage through configuration files or environment variables
- Accidental inclusion of keys in version control

### Data Storage

- Local data persistence and file permissions
- Storage of sensitive user configuration
- Cache or temporary file security

### Provider Communication

- TLS/HTTPS enforcement for API calls
- Request/response data exposure
- Man-in-the-middle attack vectors
- Credential transmission security

### Supply Chain

- Dependency vulnerabilities (Cargo ecosystem)
- Build-time security concerns

## Out of Scope

- Vulnerabilities in third-party LLM provider APIs themselves
- Issues in upstream dependencies (report to their maintainers)
- Social engineering attacks

## Safe Harbor

We consider security research conducted in accordance with this policy as:

- Authorized under computer fraud and abuse laws
- Exempt from DMCA restrictions
- Conducted in good faith

We will not pursue legal action against researchers who follow this policy.

## Preferred Languages

We accept vulnerability reports in English or Spanish.
