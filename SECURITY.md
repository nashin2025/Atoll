# Atoll Protocol - Security Baseline

## Overview

Phase 0 security baseline establishes foundational security practices for the Atoll Protocol. All secrets are kept out of version control, dependencies are audited for known vulnerabilities, and security review processes are defined.

## 1. Secret Management

### .env.example
All environment variables are documented in `.env.example` with descriptions and safe defaults. No real values are committed.

### .gitignore
The following are excluded from version control:
- `.env`, `.env.local`, `.env.production`, `.env.development`
- `*.log`
- `codebase-memory/`

### Secret Scanning
Before any commit, staged files are scanned for:
- Hardcoded API keys, passwords, tokens, connection strings
- Private keys (RSA, EC, Dilithium, SPHINCS+)
- AWS/Azure/GCP credentials
- Database connection strings with credentials

If secrets are found:
1. Remove from file
2. Add to `.gitignore`
3. Rotate the exposed secret
4. Warn the security lead

## 2. Dependency Audit

### cargo-audit
Rust dependencies are scanned for known vulnerabilities using `cargo-audit` in CI.

### CI Integration
The `.github/workflows/ci.yml` includes a security-scan job that runs `cargo audit` on every push and pull request.

### Vulnerability Response
| Severity | Response Time | Action |
|----------|--------------|--------|
| Critical | 24 hours | Patch or dependency update immediately |
| High | 72 hours | Patch or dependency update within 3 days |
| Medium | 1 week | Evaluate and patch or accept risk |
| Low | 1 month | Track in backlog |

## 3. Security Review Process

### Pre-Commit
- Secret scanning on staged files
- Dependency vulnerability check
- Code review for security patterns

### Pre-PR
- Full security review by Sentinel (Security Guard)
- SAST scanning on changed files
- Accessibility audit if frontend changes

### Pre-Merge
- All security findings addressed or formally accepted
- Audit trail maintained in SECURITY.md

## 4. Security Documentation

| Document | Purpose | Location |
|----------|---------|----------|
| SECURITY.md | Security baseline and policies | Root |
| SECURITY_AUDIT_PLAN.md | Audit scope and schedule | `audits/` |
| Threat model | STRIDE analysis | `audits/threat-model.md` |

## 5. Current Status

- [x] `.env.example` created with all environment variables documented
- [x] `.gitignore` excludes secrets and logs
- [x] CI/CD pipeline includes `cargo audit` security scan
- [x] Secret scanning process defined
- [ ] Initial `cargo audit` run (in progress)
- [ ] Threat model documentation (Phase 1)
- [ ] Security audit scheduling (Phase 9)

## 6. Known Vulnerabilities

None identified at baseline. Dependencies are at latest compatible versions.

## 7. Contact

Security issues should be reported through the bug bounty program (see specification/research/roadmap.md for bounty tiers).
