import { defineConfig } from 'vitest/config';
import tsconfigPaths from 'vite-tsconfig-paths';

export default defineConfig({
  plugins: [tsconfigPaths()],
  test: {
    environment: 'node',
    testTimeout: 30000,
    hookTimeout: 30000,
    // Run tests sequentially to avoid bankrun conflicts
    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: false,
        maxForks: 10,
      },
    },
    // Include test files
    include: ['tests/**/*.spec.ts', 'tests/**/*.test.ts'],
    // Globals for Jest-like API (optional, but makes migration easier)
    globals: true,
  },
});
