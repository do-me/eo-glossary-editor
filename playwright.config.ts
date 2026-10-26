import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: 'tests',
  fullyParallel: false,
  workers: 1,
  timeout: 45000,
  use: {
    baseURL: 'http://127.0.0.1:1420',
    viewport: { width: 1320, height: 860 },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: [
    {
      command:
        'mkdir -p .cache && cargo run -p eo-editor-cli --features test-server -- --database .cache/e2e.sqlite3 serve --port 1421 --test-token local-e2e-only',
      url: 'http://127.0.0.1:1421',
      reuseExistingServer: !process.env.CI,
      timeout: 180000,
    },
    {
      command:
        'VITE_CORE_ENDPOINT=http://127.0.0.1:1421 VITE_TEST_TOKEN=local-e2e-only npm run dev',
      url: 'http://127.0.0.1:1420',
      reuseExistingServer: !process.env.CI,
      timeout: 60000,
    },
  ],
});
