import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: [
      "marker/packages/*/test/**/*.test.ts",
      "marker/apps/*/src/**/*.test.{ts,tsx}",
    ],
  },
});
