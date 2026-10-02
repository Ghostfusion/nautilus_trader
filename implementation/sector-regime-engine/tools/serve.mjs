#!/usr/bin/env bun
/**
 * Minimal static server for the app. Optional: index.html also works opened directly from disk,
 * because the bundle is an IIFE rather than an ES module.
 *
 * Usage:  bun run serve        then open http://127.0.0.1:8787
 */

import { extname, join, normalize } from 'node:path';

const root = normalize(join(import.meta.dir, '..'));
const port = Number(process.env.PORT ?? 8787);

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.csv': 'text/csv; charset=utf-8',
  '.svg': 'image/svg+xml',
};

const server = Bun.serve({
  hostname: '127.0.0.1',
  port,
  async fetch(request) {
    const url = new URL(request.url);
    const requested = url.pathname === '/' ? '/index.html' : url.pathname;
    const resolved = normalize(join(root, requested));
    if (!resolved.startsWith(root)) return new Response('forbidden', { status: 403 });
    const file = Bun.file(resolved);
    if (!(await file.exists())) return new Response('not found', { status: 404 });
    return new Response(file, {
      headers: { 'content-type': MIME[extname(resolved)] ?? 'application/octet-stream' },
    });
  },
});

console.log(`serving ${root} at http://127.0.0.1:${server.port}`);
