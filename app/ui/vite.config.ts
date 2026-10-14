import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// `pnpm dev` proxies the app's commands to the development server
// (`cargo run -p ffx-studio --example serve`).
export default defineConfig({
  plugins: [svelte()],
  base: './',
  server: { port: 5173, proxy: { '/api': 'http://127.0.0.1:8787' } },
  clearScreen: false,
})
