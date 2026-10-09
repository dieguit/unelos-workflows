import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';

test('package contains native entrypoints, not worker orchestration or shell helper',()=>{
  const [archive]=JSON.parse(execFileSync('npm',['pack','--dry-run','--json'],{cwd:new URL('../',import.meta.url),encoding:'utf8',stdio:['ignore','pipe','pipe']}));
  assert.deepEqual(archive.files.map(f=>f.path).sort(),['README.md','commands/unelos.md','index.js','package.json','v1.js']);
});
