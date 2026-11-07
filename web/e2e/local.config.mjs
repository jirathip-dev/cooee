import { devices } from "@playwright/test";
import base from "./playwright.config.mjs";
export default { ...base, projects: base.projects.filter((p) => p.name !== "webkit").map((p) => (p.name === "pair" ? { ...p, use: { ...devices["Desktop Chrome"], colorScheme: "dark" } } : p)) };
