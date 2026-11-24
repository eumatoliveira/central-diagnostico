import { execFile } from 'node:child_process';

export const maxScanBytes = 24 * 1024 * 1024;

export function scanBuffer(bytes, execute = execFile) {
  return new Promise((resolve, reject) => {
    const child = execute('/usr/bin/clamscan', ['--no-summary', '-'], {
      timeout: 60_000,
      maxBuffer: 32_768,
    }, (error) => {
      if (!error) resolve('sem-deteccao');
      else if (error.code === 1) resolve('detectado');
      else reject(error);
    });
    child.stdin.on('error', () => {});
    child.stdin.end(bytes);
  });
}
