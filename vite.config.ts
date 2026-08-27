import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { execFileSync } from 'node:child_process';

function buildSha(): string {
  if (process.env.BUILD_SHA && process.env.BUILD_SHA !== 'development') return process.env.BUILD_SHA;
  try { return execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(); }
  catch { return 'unversioned-build'; }
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
