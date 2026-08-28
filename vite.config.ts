import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

function buildSha(): string {
  return process.env.BUILD_SHA || 'dev';
}

export default defineConfig({
  define: { 'import.meta.env.VITE_BUILD_SHA': JSON.stringify(buildSha()) },
  plugins: [svelte()],
  root: 'frontend',
  publicDir: 'public',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    sourcemap: false
  },
  server: {
    port: 5173,
    proxy: { '/api': 'http://127.0.0.1:8080', '/health': 'http://127.0.0.1:8080' }
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts']
  }
});
