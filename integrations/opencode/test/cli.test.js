import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync, chmodSync, rmSync } from 'node:fs';
import { cliJson, readCurrent } from '../cli.js';

function fixture(t) {
  const project = mkdtempSync('/tmp/opencode/t15 cli; ');
  t.after(() => rmSync(project, { recursive: true }));
  const binary = project + '/fake cli; .cjs';
  writeFileSync(binary, `#!/usr/bin/env node
const fs=require('node:fs');
const project=process.argv[3], command=process.argv[4];
fs.appendFileSync(project+'/calls',JSON.stringify(process.argv.slice(2))+'\\n');
const value=JSON.parse(fs.readFileSync(project+'/responses.json','utf8'))[command];
if(value?.error) {console.error(value.error);process.exit(2);}
process.stdout.write(value?.raw ?? JSON.stringify(value));
`);
  chmodSync(binary, 0o755);
  return { project, binary,
    set: responses => writeFileSync(project + '/responses.json', JSON.stringify(responses)),
    calls: () => readFileSync(project + '/calls', 'utf8').trim().split('\n').map(JSON.parse) };
}
const status = { status: 'waiting', runId: 'r-000001', current: { kind: 'human-gate', nodeId: 'approve' } };
const packet = { kind: 'human-gate', runId: 'r-000001', nodeId: 'approve', activationId: 'gate-2' };

test('waiting resumes next using argv, not shell expansion or Workflow discovery', async t => {
  const f = fixture(t); f.set({ status, next: packet });
  assert.deepEqual(await readCurrent(f.binary, f.project), { status, packet });
  assert.deepEqual(f.calls(), [
    ['--project', f.project, 'status', '--json'], ['--project', f.project, 'next', '--json'],
  ]);
});
test('idle discovers choices without mutating; running and finished read next', async t => {
  const f = fixture(t);
  f.set({ status: { status: 'idle' }, start: { workflows: ['guide'] } });
  assert.deepEqual(await readCurrent(f.binary, f.project), { status: { status: 'idle' }, workflows: ['guide'] });
  for (const state of ['running', 'finished']) {
    const kind = state === 'running' ? 'task' : 'finished';
    const currentStatus = { ...status, status: state, current: { kind, nodeId: 'approve' } };
    const currentPacket = { ...packet, kind };
    f.set({ status: currentStatus, next: currentPacket });
    assert.deepEqual(await readCurrent(f.binary, f.project), { status: currentStatus, packet: currentPacket });
  }
});
test('unknown status, malformed choices and torn identities reject', async t => {
  const f = fixture(t);
  for (const responses of [
    { status: { status: 'unknown' } }, { status: { status: 'idle' }, start: { workflows: [5] } },
    ...[{ runId: 'other' }, { nodeId: 'other' }, { kind: 'task' }]
      .map(change => ({ status, next: { ...packet, ...change } })),
  ]) {
    f.set(responses);
    await assert.rejects(readCurrent(f.binary, f.project), { code: 'invalid_cli_status' });
  }
});
test('process failure and invalid JSON never leak raw output or stale success', async t => {
  const f = fixture(t); f.set({ status, next: packet });
  await readCurrent(f.binary, f.project);
  f.set({ status, next: { error: 'PRIVATE RAW ERROR' } });
  await assert.rejects(readCurrent(f.binary, f.project), error =>
    error.code === 'cli_failed' && !error.message.includes('PRIVATE'));
  f.set({ status: { error: JSON.stringify({ error: { code: 'invalid_project', message: 'Project invalid' } }) } });
  await assert.rejects(cliJson(f.binary, f.project, 'status'), { code: 'invalid_project', message: 'Project invalid' });
  for (const raw of ['invalid PRIVATE JSON', 'null', '[]', '5']) {
    f.set({ status: { raw } });
    await assert.rejects(cliJson(f.binary, f.project, 'status'), { code: 'invalid_cli_json' });
  }
});
test('mutation commands reject before spawning; same identity rereads changed bodies', async t => {
  const f = fixture(t); f.set({ status, next: packet });
  await readCurrent(f.binary, f.project);
  const calls = f.calls().length;
  await assert.rejects(cliJson(f.binary, f.project, 'complete'));
  assert.equal(f.calls().length, calls);
  f.set({ status, next: { ...packet, selectedBody: 'Changed under the same identity' } });
  assert.equal((await readCurrent(f.binary, f.project)).packet.selectedBody, 'Changed under the same identity');
});
