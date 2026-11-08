import { devices } from "@playwright/test";
import base from "./playwright.config.mjs";
// Lane only (not committed): no WebKit on this host; pair on Chromium.
export default {
  ...base,
  projects: [
    { ...base.projects[0], use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 }, colorScheme: "dark" } },
    base.projects[1],
    base.projects[2],
  ],
};
