import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { svelteTesting } from '@testing-library/svelte/vite'
import { defineConfig, type Plugin } from 'vitest/config'
import { visualizer } from 'rollup-plugin-visualizer'

/**
 * Dev-server only: stores computed-style snapshots written by `src/dev/style-snapshot.ts`
 * in `frontend/.snapshots/` so a refactor can be compared against a baseline.
 */
function snapshotStore(): Plugin {
  const dir = fileURLToPath(new URL('.snapshots', import.meta.url))
  const fileFor = (name: string | null): string | null =>
    name !== null && /^[a-z0-9_-]{1,64}$/i.test(name) ? path.join(dir, `${name}.json`) : null

  return {
    name: 'ntd-snapshot-store',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/__snapshot', (req, res) => {
        const url = new URL(req.url ?? '/', 'http://localhost')
        const file = fileFor(url.searchParams.get('name'))
        if (file === null) {
          res.statusCode = 400
          res.end('invalid name')
          return
        }
        if (url.pathname === '/save' && req.method === 'POST') {
          const chunks: Buffer[] = []
          req.on('data', (chunk: Buffer) => chunks.push(chunk))
          req.on('end', () => {
            fs.mkdirSync(dir, { recursive: true })
            fs.writeFileSync(file, Buffer.concat(chunks))
            res.end('ok')
          })
        } else if (url.pathname === '/load' && fs.existsSync(file)) {
          res.setHeader('Content-Type', 'application/json')
          res.end(fs.readFileSync(file))
        } else {
          res.statusCode = 404
          res.end('not found')
        }
      })
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    svelte(),
    svelteTesting(),
    snapshotStore(),
    visualizer({
      emitFile: true,
      filename: "stats.html",
    }) as any,
  ],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/setupTests.ts']
  }
})
