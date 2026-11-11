import { defineConfig, devices } from "@playwright/test";

const port = process.env.HERDSMAN_PORT ?? "4747";
const code = new URL("./.auth/code", import.meta.url).pathname;
const storageState = new URL("./.auth/state.json", import.meta.url).pathname;

export default defineConfig({
  testDir: ".",
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: { baseURL: `http://127.0.0.1:${port}` },
  projects: [
    { name: "pair", testMatch: /pair\.setup\.mjs/, use: { ...devices["iPhone 13"], colorScheme: "dark" } },
    { name: "390", dependencies: ["pair"], use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 }, storageState } },
    { name: "1440", dependencies: ["pair"], use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 }, storageState } },
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
