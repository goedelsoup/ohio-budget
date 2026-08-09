import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  resolve: {
    alias: {
      '~': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'node',
    // The derivation layer is what is worth testing: page components are a thin rendering
    // of it, and the facts themselves are already gated by the Rust side's own tests.
    include: ['src/**/*.test.ts'],
  },
});
