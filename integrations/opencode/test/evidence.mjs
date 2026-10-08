// Native hook/API records normalized as described below; never success flags.
// context.jsonl: {role,sessionID,blocks:string[],text,model,toolIds,captureStage}
// tool-calls.json: {sessionID,tool,input,status}[]
// verdict-input.json: {workerPacket,workerText,brief,recoverySessionIDs:string[]}
// preflight.json: {required,error:{code,message},executorWorkStarted,stateBefore,stateAfter}[]
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { CONTEXT_TAG } from '../packet.js';

const root = process.argv[2];
if (!root) throw new Error('Pass a redacted native evidence directory');
const read = name => fs.readFileSync(path.join(root, name), 'utf8');
const json = name => JSON.parse(read(name));
const verdict = json('verdict-input.json');
const main = json('main-next.json');
const before = json('next-before.json');
const after = json('next-after.json');
assert.deepEqual(after, before, 'Recovery changed the authoritative packet');
// Independent projection, rather than comparing a helper with itself.
const { ledgerPath, controllerInstructions, ...expected } = before;
assert.deepEqual(verdict.workerPacket, expected);
assert.ok(!('ledgerPath' in verdict.workerPacket));
assert.ok(!('controllerInstructions' in verdict.workerPacket));
assert.ok(verdict.workerText.includes(JSON.stringify(expected, null, 2)));
assert.ok(verdict.workerText.includes(verdict.brief));
assert.ok(!verdict.workerText.includes('UNELOS_T15_UNRELATED'));
const snapshots = read('context.jsonl').trim().split('\n').map(line => JSON.parse(line));
const controllers = snapshots.filter(row => row.role === 'active-controller');
assert.ok(controllers.length > 0, 'No active-controller request captured');
for (const item of controllers) {
  assert.equal(item.blocks.filter(text => text.startsWith(CONTEXT_TAG)).length, 1);
  assert.ok(!item.blocks.join('\n').includes('UNELOS_T15_UNRELATED'));
}
const { ledgerPath: mainLedger, controllerInstructions: mainDuties, ...mainExecutor } = main;
assert.ok(controllers.some(row => row.blocks.some(text => text.includes(JSON.stringify(mainExecutor, null, 2)))));
const child = snapshots.find(row => row.role === 'worker');
assert.ok(child, 'No worker request captured');
assert.ok(child.text.includes(JSON.stringify(expected, null, 2)));
assert.ok(child.text.includes('UNELOS_T15_REVIEW_SELECTED'));
assert.ok(!child.text.includes('CONTROLLER_PRIVATE_MARKER'));
assert.ok(!child.text.includes(ledgerPath));
assert.ok(!child.blocks.some(text => text.startsWith(CONTEXT_TAG)));
assert.deepEqual(json('state-after.json'), json('state-before.json'));
for (const name of ['outline', 'review']) {
  assert.deepEqual(fs.readFileSync(path.join(root, name + '-after.md')),
    fs.readFileSync(path.join(root, name + '-before.md')));
}
const brief = text => {
  const section = text.split(/^## Brief[ \t]*\r?$/m)[1];
  assert.ok(section, 'Ledger has no Brief section');
  return section.split(/^##[ \t]+/m)[0].trim();
};
const savedBrief = brief(read('ledger-before.md'));
assert.ok(savedBrief);
assert.equal(brief(read('ledger-after.md')), savedBrief);
assert.equal(verdict.brief, savedBrief);
assert.ok(verdict.recoverySessionIDs.length > 0);
const calls = json('tool-calls.json');
const calledPath = call => call.input?.path ?? call.input?.filePath;
assert.ok(calls.some(call => verdict.recoverySessionIDs.includes(call.sessionID)
  && call.tool === 'read' && call.status === 'completed' && calledPath(call) === ledgerPath));
assert.ok(controllers.some(row => verdict.recoverySessionIDs.includes(row.sessionID)
  && row.text.includes(savedBrief)));
const checklist = before.skills.find(skill => skill.id === 'readability').directory + '/checklist.md';
assert.ok(calls.some(call => call.sessionID === child.sessionID && call.tool === 'read'
  && call.status === 'completed' && calledPath(call) === checklist));
const ordinary = snapshots.filter(row => row.role === 'ordinary');
assert.ok(ordinary.length > 0);
assert.ok(ordinary.every(row => !row.blocks.some(text => text.startsWith(CONTEXT_TAG))));
const rejected = json('preflight.json');
for (const required of ['model', 'tools', 'mcps']) {
  const probe = rejected.find(item => item.required === required);
  assert.ok(probe, 'Missing requirement-rejection evidence: ' + required);
  assert.ok(probe.error.code);
  assert.equal(probe.executorWorkStarted, false);
  assert.deepEqual(probe.stateAfter, probe.stateBefore);
}
console.log('Native T15 delivery evidence passed');
