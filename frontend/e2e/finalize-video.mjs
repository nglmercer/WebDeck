// Post-process the Playwright demo recording: transcode the native WebM to
// a broadly compatible H.264 MP4 when ffmpeg is available, else keep WebM.
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const webm = path.join(here, '..', 'demo', 'webdeck-demo-720p.webm');
const mp4 = path.join(here, '..', 'demo', 'webdeck-demo-720p.mp4');

if (!existsSync(webm)) {
  console.error(`demo recording not found: ${webm}`);
  process.exit(1);
}

try {
  execFileSync('ffmpeg', ['-version'], { stdio: 'ignore' });
} catch {
  console.log(`ffmpeg not found; keeping ${webm}`);
  process.exit(0);
}

execFileSync(
  'ffmpeg',
  [
    '-y',
    '-i',
    webm,
    '-c:v',
    'libx264',
    '-pix_fmt',
    'yuv420p',
    '-crf',
    '20',
    '-movflags',
    '+faststart',
    mp4,
  ],
  { stdio: 'inherit' }
);
console.log(`demo video: ${mp4}`);
