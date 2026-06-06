// Renders the motion demo frame-by-frame via headless Chromium, then
// assembles docs/assets/demo.{mp4,gif} with ffmpeg.
//   bun run demo:motion
import { chromium } from 'playwright'
import { execSync } from 'node:child_process'
import { mkdirSync, rmSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const FPS = 15
const DURATION = 17 // seconds
const FRAMES = FPS * DURATION

const here = dirname(fileURLToPath(import.meta.url))
const framesDir = '/tmp/mainbound-demo-frames'
const outDir = join(here, '../../docs/assets')

rmSync(framesDir, { recursive: true, force: true })
mkdirSync(framesDir, { recursive: true })
mkdirSync(outDir, { recursive: true })

const browser = await chromium.launch()
const page = await browser.newPage({
  viewport: { width: 1200, height: 750 },
  deviceScaleFactor: 2
})
await page.goto('file://' + join(here, 'demo.html'))
await page.waitForTimeout(500) // fonts/images settle

process.stdout.write(`rendering ${FRAMES} frames `)
for (let f = 0; f < FRAMES; f++) {
  await page.evaluate(t => window.renderFrame(t), f / FPS)
  await page.screenshot({ path: `${framesDir}/f${String(f).padStart(4, '0')}.png` })
  if (f % FPS === 0) process.stdout.write('.')
}
console.log(' done')
await browser.close()

console.log('encoding mp4…')
execSync(`ffmpeg -y -loglevel error -framerate ${FPS} -i "${framesDir}/f%04d.png" \
  -c:v libx264 -pix_fmt yuv420p -crf 20 -movflags +faststart "${outDir}/demo.mp4"`)

console.log('encoding gif…')
execSync(`ffmpeg -y -loglevel error -framerate ${FPS} -i "${framesDir}/f%04d.png" \
  -vf "scale=1200:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=bayer" \
  "${outDir}/demo.gif"`)

execSync(`du -h "${outDir}/demo.mp4" "${outDir}/demo.gif"`, { stdio: 'inherit' })
