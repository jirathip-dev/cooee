import { defineConfig, devices } from "@playwright/test";

// A lane's $HERDSMAN_PORT locally; 4747 in CI.
const port = process.env.HERDSMAN_PORT ?? "4747";
// The `pair` project pairs once with serve.sh's code; the others reuse its cookie.
const code = new URL("./.auth/code", import.meta.url).pathname;
const storageState = new URL("./.auth/state.json", import.meta.url).pathname;

export default defineConfig({
  testDir: ".",
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    // WebKit, as on the owner's iPhone; the cookie it saves works in Chromium too.
    { name: "pair", testMatch: /pair\.setup\.mjs/, use: { ...devices["iPhone 13"], colorScheme: "dark" } },
    { name: "390", dependencies: ["pair"], use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 }, storageState } },
    { name: "1440", dependencies: ["pair"], use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 }, storageState } },
    // The iPhone audits (#24: keyboard, sheets) on WebKit.
    { name: "webkit", dependencies: ["pair"], testMatch: /reliability\.spec\.mjs/, use: { ...devices["iPhone 13"], colorScheme: "dark", storageState } },
  ],
  webServer: {
    command: "./serve.sh",
    url: `http://127.0.0.1:${port}/`,
    env: { PORT: port, PAIR_CODE: code },
    timeout: 600_000,
    reuseExistingServer: false,
  },
});
