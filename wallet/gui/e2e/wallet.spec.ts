import { test, expect } from './fixtures';

test.describe('Wallet Creation', () => {
  test('should create a new wallet and unlock it', async ({ page, createWallet }) => {
    await page.goto('/');

    // Click "Create New Wallet"
    await page.getByRole('button', { name: /create new wallet/i }).click();

    // Enter passphrase
    await page.getByLabel(/passphrase/i).fill('test-passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('test-passphrase-123');

    // Submit
    await page.getByRole('button', { name: /create/i }).click();

    // Should navigate to dashboard
    await expect(page).toHaveURL('/dashboard');
    await expect(page.getByText(/dashboard/i)).toBeVisible();
  });

  test('should reject mismatched passphrases', async ({ page }) => {
    await page.goto('/');

    await page.getByRole('button', { name: /create new wallet/i }).click();

    await page.getByLabel(/passphrase/i).fill('passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('different-passphrase');

    await page.getByRole('button', { name: /create/i }).click();

    // Should show error
    await expect(page.getByText(/passwords do not match/i)).toBeVisible();
  });

  test('should reject empty passphrase', async ({ page }) => {
    await page.goto('/');

    await page.getByRole('button', { name: /create new wallet/i }).click();

    await page.getByLabel(/passphrase/i).fill('');
    await page.getByLabel(/confirm passphrase/i).fill('');

    await page.getByRole('button', { name: /create/i }).click();

    // Should show error
    await expect(page.getByText(/passphrase is required/i)).toBeVisible();
  });
});

test.describe('Wallet Import', () => {
  test('should import a wallet from secret key', async ({ page }) => {
    await page.goto('/');

    await page.getByRole('button', { name: /import existing key/i }).click();

    // Enter a valid 32-byte hex secret key
    await page.getByLabel(/secret key/i).fill('03'.repeat(32));
    await page.getByLabel(/passphrase/i).fill('import-passphrase-123');

    await page.getByRole('button', { name: /import/i }).click();

    await expect(page).toHaveURL('/dashboard');
  });

  test('should reject invalid secret key length', async ({ page }) => {
    await page.goto('/');

    await page.getByRole('button', { name: /import existing key/i }).click();

    await page.getByLabel(/secret key/i).fill('deadbeef');
    await page.getByLabel(/passphrase/i).fill('passphrase-123');

    await page.getByRole('button', { name: /import/i }).click();

    await expect(page.getByText(/invalid/i)).toBeVisible();
  });
});

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    // Create and unlock a wallet before each test
    await page.goto('/');
    await page.getByRole('button', { name: /create new wallet/i }).click();
    await page.getByLabel(/passphrase/i).fill('test-passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('test-passphrase-123');
    await page.getByRole('button', { name: /create/i }).click();
    await expect(page).toHaveURL('/dashboard');
  });

  test('should display address and balance', async ({ page }) => {
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible();
    await expect(page.getByText(/address/i)).toBeVisible();
    await expect(page.getByText(/balance/i)).toBeVisible();
  });

  test('should refresh balance', async ({ page }) => {
    await page.getByRole('button', { name: /refresh balance/i }).click();
    await expect(page.getByText(/balance/i)).toBeVisible();
  });

  test('should navigate to send page', async ({ page }) => {
    await page.getByRole('button', { name: /send atoll/i }).click();
    await expect(page).toHaveURL('/send');
  });

  test('should navigate to history page', async ({ page }) => {
    await page.getByRole('button', { name: /history/i }).click();
    await expect(page).toHaveURL('/history');
  });

  test('should navigate to derive address page', async ({ page }) => {
    await page.getByRole('button', { name: /derive address/i }).click();
    await expect(page).toHaveURL('/derive-address');
  });
});

test.describe('Send ATOLL', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /create new wallet/i }).click();
    await page.getByLabel(/passphrase/i).fill('test-passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('test-passphrase-123');
    await page.getByRole('button', { name: /create/i }).click();
    await page.getByRole('button', { name: /send atoll/i }).click();
    await expect(page).toHaveURL('/send');
  });

  test('should show validation errors for empty fields', async ({ page }) => {
    await page.getByRole('button', { name: /sign transaction/i }).click();
    await expect(page.getByText(/required/i)).toBeVisible();
  });

  test('should reject zero amount', async ({ page }) => {
    await page.getByLabel(/to address/i).fill('0x1234567890123456789012345678901234567890');
    await page.getByLabel(/amount/i).fill('0');
    await page.getByRole('button', { name: /sign transaction/i }).click();
    await expect(page.getByText(/greater than zero/i)).toBeVisible();
  });

  test('should sign transaction successfully', async ({ page }) => {
    await page.getByLabel(/to address/i).fill('0x1234567890123456789012345678901234567890');
    await page.getByLabel(/amount/i).fill('10');
    await page.getByRole('button', { name: /sign transaction/i }).click();
    await expect(page.getByText(/signed transaction hex/i)).toBeVisible();
  });
});

test.describe('Derive Address', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /create new wallet/i }).click();
    await page.getByLabel(/passphrase/i).fill('test-passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('test-passphrase-123');
    await page.getByRole('button', { name: /create/i }).click();
    await page.getByRole('button', { name: /derive address/i }).click();
    await expect(page).toHaveURL('/derive-address');
  });

  test('should derive address from public key', async ({ page }) => {
    await page.getByLabel(/public key/i).fill('03'.repeat(32));
    await page.getByRole('button', { name: /derive address/i }).click();
    await expect(page.getByText(/derived address/i)).toBeVisible();
  });

  test('should reject invalid public key length', async ({ page }) => {
    await page.getByLabel(/public key/i).fill('deadbeef');
    await page.getByRole('button', { name: /derive address/i }).click();
    await expect(page.getByText(/invalid/i)).toBeVisible();
  });
});

test.describe('Node Connection', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /create new wallet/i }).click();
    await page.getByLabel(/passphrase/i).fill('test-passphrase-123');
    await page.getByLabel(/confirm passphrase/i).fill('test-passphrase-123');
    await page.getByRole('button', { name: /create/i }).click();
    await expect(page).toHaveURL('/dashboard');
  });

  test('should connect to node', async ({ page }) => {
    await page.getByRole('button', { name: /connect node/i }).click();
    await page.getByLabel(/node url/i).fill('http://localhost:8081');
    await page.getByRole('button', { name: /connect/i }).click();
    await expect(page.getByText(/connected/i)).toBeVisible();
  });

  test('should show node status', async ({ page }) => {
    await page.getByRole('button', { name: /connect node/i }).click();
    await page.getByLabel(/node url/i).fill('http://localhost:8081');
    await page.getByRole('button', { name: /connect/i }).click();
    await expect(page.getByText(/tip height/i)).toBeVisible();
    await expect(page.getByText(/peer count/i)).toBeVisible();
  });
});
