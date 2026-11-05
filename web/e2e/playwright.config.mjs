import { defineConfig, devices } from "@playwright/test";

// A lane's $HERDSMAN_PORT locally; 4747 in CI.
const port = process.env.HERDSMAN_PORT ?? "4747";

export default defineConfig({
  testDir: ".",
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    { name: "390", use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 } } },
    { name: "1440", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } },
  ],
  webServer: {
    command: "./serve.sh",
    url: `http://127.0.0.1:${port}/`,
    env: { PORT: port },
    timeout: 600_000,
    reuseExistingServer: false,
  },
});
