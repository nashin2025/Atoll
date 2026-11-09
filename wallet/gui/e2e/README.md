# Atoll GUI Wallet E2E Tests

# End-to-End Tests for Atoll GUI Wallet

This directory contains E2E tests for the Atoll GUI wallet using Playwright and Vitest.

## Test Coverage

- Wallet creation (new, import)
- Dashboard (balance, address, navigation)
- Send ATOLL (signing, validation)
- Derive Address (public key → address)
- Node connection (connect, status)

## Test Architecture

Tests use Playwright with custom fixtures for wallet operations:
- Each test runs in isolation
- Wallets are created fresh for each test
- Assertions check both UI state and on-chain data
- Error cases are tested (invalid inputs, empty fields, etc.)

## Running Tests

### Prerequisites

- Node.js 20+
- The GUI wallet dev server running (`npm run dev`)

### Run All E2E Tests

```bash
# Install Playwright browsers (first time only)
npx playwright install

# Run E2E tests
npm run test:e2e

# Run with UI mode
npx playwright test --ui

# Run specific test file
npx playwright test e2e/wallet.spec.ts

# Run in headed mode (see browser)
npx playwright test --headed
```

### Add E2E Script to package.json

Add this to `package.json` scripts:

```json
"test:e2e": "playwright test"
```

## Test Structure

```
e2e/
├── fixtures.ts          # Test fixtures and helpers
└── wallet.spec.ts       # Wallet E2E tests
```

## Best Practices Followed

1. **Atomic tests**: Each test is independent and self-contained
2. **State control**: Wallets created fresh for each test via fixtures
3. **Real transactions**: Tests sign and submit real transactions
4. **On-chain verification**: Assertions check both UI and on-chain state
5. **Error cases**: Invalid inputs, empty fields, and error states are tested
6. **No hardcoded secrets**: Test passphrases and keys are test-only
7. **Isolation**: Each test runs with a dedicated browser context

## Writing New Tests

1. Add test cases to `wallet.spec.ts` or create new `.spec.ts` files
2. Use the `test` fixture from `./fixtures`
3. Follow the Arrange-Act-Assert pattern
4. Test both happy paths and error cases
5. Assert on-chain state, not just UI rendering
