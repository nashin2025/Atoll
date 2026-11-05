# Atoll Protocol - Security Policy

## 🔒 Supported Versions

| Version | Supported          |
|---------|--------------------|
| v0.1.x  | :white_check_mark: |
| < v0.1  | :x:                |

## 📋 Reporting a Vulnerability

We take the security of Atoll Protocol seriously. If you discover a security vulnerability, please follow these steps:

### 1. Do NOT Disclose Publicly

Do not create a public GitHub issue or discuss the vulnerability in public forums.

### 2. Report Privately

Send details to [security@atollprotocol.org](mailto:security@atollprotocol.org) with:

- **Title**: Clear description of the vulnerability
- **Description**: Detailed explanation of the issue
- **Steps to Reproduce**: Step-by-step instructions
- **Impact**: Potential impact and affected components
- **Suggested Fix**: If you have one
- **Your Contact Information**: For follow-up

### 3. What to Expect

- **Acknowledgment**: Within 24 hours
- **Initial Assessment**: Within 72 hours
- **Resolution**: Within 7 days for critical issues, 30 days for others
- **Credit**: You will be credited in our security advisories (if desired)

## 🛡️ Security Baseline

### 1. Secret Management

All secrets are kept out of version control. Environment variables are documented in `.env.example` with safe defaults.

**Protected patterns:**
- Hardcoded API keys, passwords, tokens, connection strings
- Private keys (RSA, EC, Dilithium, SPHINCS+)
- AWS/Azure/GCP credentials
- Database connection strings with credentials

**If secrets are found:**
1. Remove from file
2. Add to `.gitignore`
3. Rotate the exposed secret
4. Warn the security lead

### 2. Dependency Audit

Rust dependencies are scanned for known vulnerabilities using `cargo-audit` in CI.

**Vulnerability Response:**

| Severity | Response Time | Action |
|----------|--------------|--------|
| Critical | 24 hours | Patch or dependency update immediately |
| High | 72 hours | Patch or dependency update within 3 days |
| Medium | 1 week | Evaluate and patch or accept risk |
| Low | 1 month | Track in backlog |

### 3. Security Review Process

#### Pre-Commit
- Secret scanning on staged files
- Dependency vulnerability check
- Code review for security patterns

#### Pre-PR
- Full security review by Sentinel (Security Guard)
- SAST scanning on changed files
- Accessibility audit if frontend changes

#### Pre-Merge
- All security findings addressed or formally accepted
- Audit trail maintained in SECURITY.md

## 📄 Security Documentation

| Document | Purpose | Location |
|----------|---------|----------|
| SECURITY.md | Security baseline and policies | Root |
| SECURITY_AUDIT_PLAN.md | Audit scope and schedule | `audits/` |
| Threat model | STRIDE analysis | `audits/threat-model.md` |
| Security advisories | Published vulnerabilities | `docs/security/` |

## 🔐 Security Features

- **Post-quantum signatures** (Dilithium2/3/5, SPHINCS+)
- **Post-quantum KEM** (Kyber512/768/1024)
- **Hybrid signature engine** (classical + PQ)
- **Homomorphic encryption** (BFV/CKKS)
- **zk-STARKs** (zero-knowledge proofs)
- **Encrypted mempool** (threshold decryption)
- **Encrypted keystore** (XChaCha20-Poly1305 + argon2id)
- **Noise Protocol** (Noise_XX_25519_ChaChaPoly_BLAKE2s)
- **Rate limiting** (per-IP, configurable)
- **CORS whitelist**
- **Security headers** (HSTS, CSP, X-Frame-Options)

## 📞 Contact

Security issues should be reported to [security@atollprotocol.org](mailto:security@atollprotocol.org).

For non-security issues, please use GitHub Issues or GitHub Discussions.

## 🏆 Security Researchers

We recognize security researchers who responsibly disclose vulnerabilities:

- Listed in our security hall of fame
- Mentioned in release notes
- Swag and recognition
