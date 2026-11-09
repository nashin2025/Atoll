# Atoll Protocol Disaster Recovery Runbooks

# Disaster Recovery Runbooks

This document contains detailed runbooks for common disaster scenarios.

## Runbook 1: Node Recovery

### Scenario
A validator node has gone offline and is not producing blocks.

### Prerequisites
- SSH access to the node
- Backup of node data
- Access to block explorer or another node for sync status

### Steps

1. **Identify the issue**
   ```bash
   # Check node status
   docker compose -f docker-compose.testnet.yml ps
   
   # Check node logs
   docker logs atoll-node
   ```

2. **Restart the node**
   ```bash
   docker restart atoll-node
   ```

3. **Verify sync**
   ```bash
   curl http://localhost:8081/net_peerCount
   curl http://localhost:8081/eth_blockNumber
   ```

4. **If sync fails, restore from backup**
   ```bash
   docker compose -f docker-compose.testnet.yml down
   docker volume rm atoll-testnet-data
   docker compose -f docker-compose.testnet.yml up -d
   ```

5. **Verify sync completion**
   ```bash
   # Wait for block height to catch up
   curl http://localhost:8081/eth_blockNumber
   ```

---

## Runbook 2: Key Compromise

### Scenario
A private key has been compromised and unauthorized transactions are occurring.

### Prerequisites
- Access to multi-sig wallet
- List of all addresses controlled by compromised key
- List of all transactions signed by compromised key

### Steps

1. **Revoke compromised key**
   ```bash
   # Use multi-sig to revoke key
   # This requires 3 of 5 signer approval
   ```

2. **Rotate all related keys**
   ```bash
   # Generate new keys
   # Update all configurations
   # Redeploy contracts with new admin keys
   ```

3. **Audit affected transactions**
   ```bash
   # Query blockchain for all transactions signed by compromised key
   # Identify affected addresses
   # Calculate funds at risk
   ```

4. **Recover funds if possible**
   ```bash
   # Use multi-sig to transfer funds to new addresses
   # Coordinate with exchanges if funds were sent there
   ```

5. **Report to authorities if required**
   ```bash
   # File report with relevant authorities
   # Notify affected users
   ```

---

## Runbook 3: Chain Halt

### Scenario
The blockchain has stopped producing new blocks.

### Prerequisites
- Access to validator nodes
- Access to governance multi-sig
- Communication with validator operators

### Steps

1. **Identify the issue**
   ```bash
   # Check block height across all nodes
   # Identify which nodes are still producing blocks
   ```

2. **Pause contracts if funds at risk**
   ```bash
   # Use multi-sig to pause critical contracts
   ```

3. **Coordinate with validators**
   ```bash
   # Contact all validator operators
   # Identify root cause of halt
   # Agree on restart procedure
   ```

4. **Resume operations**
   ```bash
   # Restart all validator nodes
   # Verify block production resumes
   # Verify transaction finality
   ```

5. **Resume contract operations**
   ```bash
   # Unpause contracts via multi-sig
   # Verify normal operation
   ```

---

## Runbook 4: Data Corruption

### Scenario
Blockchain data is corrupted or inconsistent.

### Prerequisites
- Backup of node data
- Access to another node for sync
- Time to resync

### Steps

1. **Stop node operations**
   ```bash
   docker compose -f docker-compose.testnet.yml down
   ```

2. **Identify corruption**
   ```bash
   # Check block hashes against another node
   # Identify corrupted blocks
   ```

3. **Restore from backup**
   ```bash
   # Restore from latest backup
   docker volume rm atoll-testnet-data
   docker compose -f docker-compose.testnet.yml up -d
   ```

4. **Resync from network**
   ```bash
   # Node will resync from peers
   # Verify block height matches network
   ```

5. **Verify data integrity**
   ```bash
   # Check block hashes
   # Verify transaction history
   ```

---

## Runbook 5: Key Rotation

### Scenario
Scheduled key rotation for security hygiene.

### Steps

1. **Generate new keys**
   ```bash
   # Generate new key pairs
   # Store securely in HSM
   ```

2. **Update configurations**
   ```bash
   # Update node configurations with new keys
   # Update multi-sig signers
   ```

3. **Test new keys**
   ```bash
   # Test signing with new keys
   # Verify transactions work
   ```

4. **Rotate keys**
   ```bash
   # Use multi-sig to update admin keys
   # Revoke old keys
   ```

5. **Verify rotation**
   ```bash
   # Verify old keys no longer work
   # Verify new keys work correctly
   ```

---

## Appendix: Emergency Contact Template

```
INCIDENT: [SEVERITY]
TIME: [UTC TIME]
REPORTER: [NAME]
DESCRIPTION: [WHAT HAPPENED]
IMPACT: [USERS, FUNDS, DOWNTIME]
ACTIONS TAKEN: [WHAT WAS DONE]
NEXT UPDATE: [TIME]
CONTACT: [EMERGENCY EMAIL/PHONE]
```

---

*Document version 1.0 — November 2026*
