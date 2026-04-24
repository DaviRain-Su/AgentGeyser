import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';

const STUB = JSON.stringify({ jsonrpc: '2.0', id: 1, result: '5'.repeat(64) });

export default defineConfig({
  root: resolve(__dirname),
  define: {
    __AG_SRC_ATA__: JSON.stringify(process.env.SRC_ATA || ''),
    __AG_DST_ATA__: JSON.stringify(process.env.DST_ATA || ''),
    __AG_SRC_OWNER__: JSON.stringify(process.env.SRC_OWNER || ''),
  },
  plugins: [react(), {
    name: 'stub-rpc',
    configureServer: (s) => { s.middlewares.use('/stub-rpc', (_req, res) => { res.setHeader('content-type', 'application/json'); res.end(STUB); }); },
  }],
  server: {
    host: '127.0.0.1', port: 5173, strictPort: true,
    proxy: { '/ag-proxy': { target: 'http://127.0.0.1:8999', changeOrigin: true, rewrite: (p) => p.replace(/^\/ag-proxy/, '') } },
  },
});
