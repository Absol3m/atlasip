import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath, URL } from 'url';

// Standalone vitest config that uses @sveltejs/vite-plugin-svelte directly
// (instead of sveltekit()) so that Svelte components resolve to their browser
// build rather than the SSR build during tests.
export default defineConfig({
  plugins: [
    svelte({ hot: false }),
  ],
  resolve: {
    // Force Svelte (and its deps) to resolve to browser builds rather than the
    // default SSR build.  Without 'browser' here, `import * as Svelte from 'svelte'`
    // inside @testing-library/svelte-core resolves to index-server.js and `mount`
    // throws "not available on the server".
    conditions: ['browser'],
    alias: {
      // Mirror SvelteKit's $lib alias
      '$lib': fileURLToPath(new URL('src/lib', import.meta.url)),
    },
  },
  test: {
    include: ['src/**/*.test.ts'],
    // Default to node; individual test files use // @vitest-environment happy-dom
    environment: 'node',
  },
});
