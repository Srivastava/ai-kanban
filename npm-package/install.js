#!/usr/bin/env node
const https = require('https');
const fs = require('fs');
const path = require('path');

const REPO = 'Srivastava/ai-kanban';
const BIN_DIR = path.join(__dirname, 'bin');

function getBinaryName() {
  const { platform, arch } = process;
  if (platform === 'linux' && arch === 'x64') return 'ai-kanban-linux-x86_64';
  if (platform === 'darwin' && arch === 'x64') return 'ai-kanban-macos-x86_64';
  if (platform === 'darwin' && arch === 'arm64') return 'ai-kanban-macos-arm64';
  if (platform === 'win32' && arch === 'x64') return 'ai-kanban-windows-x86_64.exe';
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        file.close();
        fs.unlinkSync(dest);
        return download(res.headers.location, dest).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} downloading binary`));
      }
      res.pipe(file);
      file.on('finish', () => file.close(resolve));
    }).on('error', (err) => {
      fs.unlinkSync(dest);
      reject(err);
    });
  });
}

async function main() {
  const binaryName = getBinaryName();
  const url = `https://github.com/${REPO}/releases/latest/download/${binaryName}`;
  const dest = path.join(BIN_DIR, process.platform === 'win32' ? 'ai-kanban.exe' : 'ai-kanban');

  if (!fs.existsSync(BIN_DIR)) fs.mkdirSync(BIN_DIR, { recursive: true });

  console.log(`Downloading ai-kanban (${process.platform}/${process.arch})...`);
  await download(url, dest);

  if (process.platform !== 'win32') fs.chmodSync(dest, 0o755);

  console.log('Done. Run: ai-kanban');
}

main().catch((err) => {
  console.error('Install failed:', err.message);
  process.exit(1);
});
