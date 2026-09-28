// Assemble the 3D tour GIF from the captured frames (two-pass palette
// for quality), then remove the working frames directory.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, rmSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const framesDir = path.join(here, '..', 'demo', '.frames-3d');
const palette = path.join(framesDir, 'palette.png');
const gif = path.join(here, '..', 'demo', '3d-tour.gif');

if (!existsSync(framesDir) || readdirSync(framesDir).length === 0) {
  console.error(`tour frames not found: ${framesDir}`);
  process.exit(1);
}

try {
  execFileSync('ffmpeg', ['-version'], { stdio: 'ignore' });
} catch {
  console.log(`ffmpeg not found; frames left in ${framesDir}`);
  process.exit(0);
}

const scale = 'scale=960:-1:flags=lanczos';
execFileSync(
  'ffmpeg',
  ['-y', '-v', 'error', '-framerate', '8', '-i', path.join(framesDir, 'f%03d.png'), '-vf', `${scale},palettegen=max_colors=128`, palette],
  { stdio: 'inherit' }
);
execFileSync(
  'ffmpeg',
  [
    '-y',
    '-v',
    'error',
    '-framerate',
    '8',
    '-i',
    path.join(framesDir, 'f%03d.png'),
    '-i',
    palette,
    '-lavfi',
    `${scale} [x]; [x][1:v] paletteuse=dither=bayer:bayer_scale=4`,
    gif,
  ],
  { stdio: 'inherit' }
);
rmSync(framesDir, { recursive: true, force: true });
console.log(`tour gif: ${gif}`);
