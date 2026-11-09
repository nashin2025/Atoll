# Atoll Protocol — Disaster Recovery Plan

## Version: 1.0
**Date:** November 2026  
**Status:** Draft  
**Network:** Atoll Mainnet (Chain ID: 777777)

---

## Table of Contents

1. [Overview](#overview)
2. [Incident Classification](#incident-classification)
3. [Rollback Procedures](#rollback-procedures)
4. [Backup Strategy](#backup-strategy)
5. [Emergency Access](#emergency-access)
6. [Incident Response Runbooks](#incident-response-runbooks)
7. [Communication Plan](#communication-plan)
8. [Post-Mortem Process](#post-mortem-process)
9. [Testing Schedule](#testing-schedule)

---

## 1. Overview

This document defines the disaster recovery plan for the Atoll Protocol mainnet. It covers rollback procedures, backup strategies, emergency access, incident response runbooks, communication plans, and post-mortem processes.

## 2. Incident Classification

| Severity | Description | Response Time | Escalation |
|----------|-------------|---------------|------------|
| **P1 - Critical** | Node down, chain halted, funds at risk | 15 minutes | On-call engineer + Core team |
| **P2 - High** | Degraded performance, high error rate | 1 hour | On-call engineer |
| **P3 - Medium** | Minor bugs, non-critical issues | 4 hours | Next business day |
| **P4 - Low** | Cosmetic issues, documentation gaps | 24 hours | Next sprint |

---

## 3. Rollback Procedures

### 3.1 Technical Rollback Steps

| Rollback Type | Procedure | When to Use |
|--------------|-----------|-------------|
| **Pause Contracts** | Activate `pause()` function on critical contracts to halt all operations | Immediate threat to funds, active exploit |
| **Feature Flags** | Disable risky features via governance-controlled feature flags | New feature causing issues |
| **Contract Upgrade** | Redeploy previous implementation via proxy contract | Bug in new contract code |
| **Role Revocation** | Revoke compromised keys or admin privileges via governance | Compromised keys, malicious actor |
| **Migration Scripts** | Run pre-prepared migration scripts to roll forward | Data corruption, state inconsistency |

### 3.2 Rollback Decision Points

Before initiating a rollback, evaluate:

1. **Can the issue be fixed forward?** (Fix and redeploy instead of rolling back)
2. **What is the blast radius?** (How many users/transactions are affected?)
3. **What is the data loss risk?** (Will rollback lose confirmed transactions?)
4. **What is the user impact?** (Will users lose funds or access?)

### 3.3 Rollback Execution

1. **Declare incident** and assign incident commander
2. **Pause operations** if funds are at risk
3. **Notify stakeholders** via communication channels
4. **Execute rollback** using the appropriate procedure
5. **Verify system state** after rollback
6. **Resume operations** when safe
7. **Document incident** in post-mortem

---

## 4. Backup Strategy

### 4.1 Database Backups

| Component | Frequency | Retention | Storage |
|-----------|-----------|-----------|---------|
| PostgreSQL (block data) | Hourly | 30 days | Encrypted S3 bucket |
| Redis (cache) | Daily | 7 days | Encrypted S3 bucket |
| Node configuration | Daily | 90 days | Encrypted S3 bucket |
| Genesis file | Once (immutable) | Permanent | Multi-sig wallet + offline |

### 4.2 Key Backups

| Key Type | Storage Location | Access Method |
|----------|-----------------|---------------|
| Node operator keys | Hardware security module (HSM) | Multi-sig required |
| Faucet keys (testnet) | Encrypted file + air-gapped backup | Multi-sig required |
| Governance keys | Multi-sig wallet (Gnosis Safe) | Threshold signature |
| Recovery keys | Offline paper backup | Physical vault |

### 4.3 Backup Verification

- **Monthly**: Restore backup to test environment
- **Quarterly**: Full disaster recovery drill
- **Annually**: Third-party audit of backup procedures

---

## 5. Emergency Access

### 5.1 Multi-Sig Emergency Wallet

An emergency multi-sig wallet (Gnosis Safe) holds:
- Emergency admin keys
- Pause/unpause authority
- Upgrade authority
- Treasury funds for emergency bounties

**Signers**: 5 independent parties  
**Threshold**: 3 of 5 required for emergency actions

### 5.2 Emergency Contact List

| Role | Name | Contact | Availability |
|------|------|---------|-------------|
| Incident Commander | TBD | emergency@atoll.network | 24/7 |
| Core Developer | TBD | dev@atoll.network | Business hours |
| Security Lead | TBD | security@atoll.network | 24/7 |
| Community Manager | TBD | community@atoll.network | Business hours |

### 5.3 Emergency Procedures

1. **Incident detected** → Notify incident commander
2. **Incident commander** → Assess severity, declare incident level
3. **Multi-sig signers** → Execute emergency actions if needed
4. **Communications** → Notify community via official channels
5. **Resolution** → Document and post-mortem

---

## 6. Incident Response Runbooks

### 6.1 Node Down

**Symptoms**: Node not responding to RPC requests, peers disconnecting

**Response**:
1. Check node logs for errors
2. Check network connectivity (firewall, port forwarding)
3. Restart node service
4. Verify block height is catching up
5. If persistent, restore from backup

### 6.2 Chain Halted

**Symptoms**: No new blocks being produced, transactions not confirming

**Response**:
1. Check validator status
2. Check consensus layer for issues
3. Execute emergency pause if funds at risk
4. Coordinate with validators for restart
5. Resume operations when safe

### 6.3 Smart Contract Bug

**Symptoms**: Funds drained, unexpected behavior, failed transactions

**Response**:
1. Pause affected contracts immediately
2. Revoke compromised admin keys
3. Assess damage and identify affected users
4. Prepare migration script if needed
5. Communicate with affected users

### 6.4 Key Compromise

**Symptoms**: Unauthorized transactions, unexpected key usage

**Response**:
1. Revoke compromised keys immediately
2. Rotate all related keys
3. Audit all actions performed by compromised key
4. Restore funds if possible via multi-sig
5. Report to authorities if required

### 6.5 Data Corruption

**Symptoms**: Invalid state, missing blocks, inconsistent data

**Response**:
1. Stop node operations
2. Identify source of corruption
3. Restore from backup
4. Verify data integrity
5. Resume operations

---

## 7. Communication Plan

### 7.1 Internal Communication

| Channel | Purpose | Audience |
|---------|---------|----------|
| Slack/Discord | Real-time incident coordination | Core team |
| Email | Formal incident notifications | All team members |
| Phone/SMS | Emergency contact | Incident commander, core team |

### 7.2 External Communication

| Channel | Purpose | Audience |
|---------|---------|----------|
| Twitter/X | Public status updates | Community |
| Discord/Telegram | Community support | Users |
| Email | Formal notices | Partners, exchanges |
| Status Page | Public status dashboard | All users |

### 7.3 Communication Templates

**Incident Declaration**:
```
🚨 INCIDENT DECLARED: [SEVERITY]
Time: [TIME]
Issue: [DESCRIPTION]
Impact: [USERS AFFECTED, FUNDS AT RISK]
Response: [ACTIONS BEING TAKEN]
Updates: [NEXT UPDATE TIME]
```

**Incident Resolution**:
```
✅ INCIDENT RESOLVED
Time: [TIME]
Issue: [DESCRIPTION]
Resolution: [WHAT WAS DONE]
Impact: [USERS AFFECTED, FUNDS LOST/RECOVERED]
Post-mortem: [LINK TO POST-MORTEM]
```

---

## 8. Post-Mortem Process

### 8.1 Post-Mortem Template

```
# Post-Mortem: [INCIDENT NAME]

**Date**: [DATE]
**Severity**: [P1/P2/P3/P4]
**Duration**: [START TIME] to [END TIME]
**Incident Commander**: [NAME]
**Responders**: [NAMES]

## Summary
[Brief description of what happened]

## Timeline
[Chronological list of events]

## Root Cause
[Root cause analysis]

## Impact
[Users affected, funds lost/recovered, downtime]

## What Went Well
[Things that worked]

## What Went Wrong
[Things that didn't work]

## Action Items
| Action | Owner | Due Date |
|--------|-------|----------|
| [Action 1] | [Name] | [Date] |
| [Action 2] | [Name] | [Date] |

## Lessons Learned
[Key takeaways]
```

### 8.2 Post-Mortem Timeline

- **Within 24 hours**: Initial post-mortem draft
- **Within 1 week**: Final post-mortem published
- **Within 1 month**: All action items completed

---

## 9. Testing Schedule

| Test | Frequency | Owner |
|------|-----------|-------|
| Backup restoration | Monthly | Operations |
| Failover drill | Quarterly | Operations |
| Full disaster recovery drill | Annually | Core team |
| Key rotation | Quarterly | Security team |
| Multi-sig key recovery | Annually | Core team + Signers |

---

## Appendix A: Emergency Contacts

| Role | Contact | Availability |
|------|---------|-------------|
| Incident Commander | emergency@atoll.network | 24/7 |
| Security Lead | security@atoll.network | 24/7 |
| Core Developer | dev@atoll.network | Business hours |
| Community Manager | community@atoll.network | Business hours |
| Legal Counsel | legal@atoll.network | Business hours |

## Appendix B: External Contacts

| Service | Contact | Purpose |
|---------|---------|---------|
| Cloud Provider | [PROVIDER] | Infrastructure support |
| Exchange | [EXCHANGE] | Trading halt if needed |
| Block Explorer | [EXPLORER] | Data verification |
| Security Firm | [FIRM] | Emergency audit |

---

*Document version 1.0 — November 2026*
