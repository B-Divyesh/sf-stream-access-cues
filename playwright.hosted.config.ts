import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/browser',
  testMatch: '**/hosted.spec.ts',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:4174',
    trace: 'retain-on-failure'
  },
  projects: [
    { name: 'hosted-desktop-chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'hosted-mobile-chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 390, height: 844 } } }
  ],
  webServer: {
    command: 'npm run build && DEPLOYMENT_MODE=hosted DATA_DIR=/tmp/stream-access-cues-e2e-hosted PORT=4174 DIST_DIR=dist cargo run',
    url: 'http://127.0.0.1:4174/health',
    reuseExistingServer: false,
    timeout: 120_000
  }
});
