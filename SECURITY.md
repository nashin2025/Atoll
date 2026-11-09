# Atoll Protocol — Security Policy

## Supported Versions

The following versions of Atoll Protocol are currently supported with security updates:

| Version | Supported          |
|---------|--------------------|
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take the security of Atoll Protocol seriously. Thank you for responsibly disclosing vulnerabilities to us.

### How to Report

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please send details to the maintainer via one of these channels:

- **GitHub Security Advisories**: [Report a vulnerability](https://github.com/nashin2025/Atoll/security/advisories/new) (preferred — private by default)
- **Email**: Contact the repository owner directly

### What to Include

Please include as much of the following as possible:

- **Type of issue** (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- **Full paths** of source file(s) related to the manifestation of the issue
- **The location** of the affected source code (tag/branch/commit or direct URL)
- **Any special configuration** required to reproduce the issue
- **Step-by-step steps** to reproduce the issue
- **Proof-of-concept or exploit code** (if possible)
- **Impact** of the issue, including how an attacker might exploit it

This information will help us triage your report more quickly.

## Response Timeline

- **Acknowledgment**: You should receive an acknowledgment within 5 business days
- **Initial assessment**: Within 10 business days we will assess the severity and provide an estimated timeline for resolution
- **Updates**: We will provide status updates at least every 7 days until the issue is resolved
- **Resolution**: We will work with you to develop and test a fix, then coordinate public disclosure

## Disclosure Policy

We follow coordinated vulnerability disclosure:

1. You report the vulnerability privately
2. We investigate and confirm the issue
3. We develop and test a fix
4. We coordinate a disclosure timeline with you
5. We publish a security advisory and credit you (if you wish)

We ask that you do not publicly disclose the vulnerability before we have had a reasonable time to address it.

## Security Best Practices for Contributors

- **Never** commit secrets, API keys, or credentials to the repository
- Use environment variables or `.env` files (which are gitignored)
- All user inputs must be validated and sanitized
- Error messages must not leak internal details
- Use parameterized queries for database operations
- Follow the principle of least privilege

## Credits

We appreciate the efforts of all security researchers who help make Atoll Protocol safer.
