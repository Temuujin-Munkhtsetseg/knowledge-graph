import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    outDir: 'dist',
  },
  server: {
    proxy: {
      '/api': {
        // This targets the KG dev-server, which is typically running on port 27495
        target: `http://localhost:${process.env.DEV_PORT || 27495}`,
        changeOrigin: true,
        secure: false,
      },
    },
  },
})
