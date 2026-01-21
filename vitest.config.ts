import { defineConfig } from 'vitest/config';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [tsconfigPaths()],
  test: {
    environment: 'node',
    testTimeout: 30000,
    hookTimeout: 30000,
    // Run tests sequentially to avoid bankrun conflicts and minimize memory usage
    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: true, // Sequential execution - minimal memory usage
      },
    },
    // Include test files
    include: ['tests/**/*.spec.ts', 'tests/**/*.test.ts'],
    // Globals for Jest-like API (optional, but makes migration easier)
    globals: true,
    // Setup file to patch bn.js for cross-platform compatibility
    setupFiles: ['./tests/setup.ts'],
  },
});
