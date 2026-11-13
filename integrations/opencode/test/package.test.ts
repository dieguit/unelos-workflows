import {test} from 'vitest';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {stripTypeScriptTypes} from 'node:module';

test('package contains native entrypoints, not worker orchestration or shell helper',()=>{
  const [archive]=JSON.parse(execFileSync('npm',['pack','--dry-run','--json'],{cwd:new URL('../',import.meta.url),encoding:'utf8',stdio:['ignore','pipe','pipe']}));
  assert.deepEqual(archive.files.map((f:{path:string})=>f.path).sort(),['README.md','commands/unelos.md','index.ts','package.json']);
});

test('installed TypeScript has no runtime SDK or shared protocol imports',()=>{
  const source=readFileSync(new URL('../index.ts',import.meta.url),'utf8');
  const runtime=stripTypeScriptTypes(source);
  assert.ok(!runtime.includes('@opencode/plugin'));
  assert.ok(!runtime.includes('../shared/protocol.ts'));
});
