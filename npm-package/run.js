#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');

const binary = path.join(
  __dirname,
  'bin',
  process.platform === 'win32' ? 'ai-kanban.exe' : 'ai-kanban'
);

const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });
child.on('exit', (code) => process.exit(code ?? 0));
