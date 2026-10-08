import test from 'node:test';
import assert from 'node:assert/strict';
import { executorPacket, executorText, controllerText, CONTEXT_TAG,
  parseModel, validateModel, requireConfiguration } from '../packet.js';

export const packet = {
  kind: 'task', runId: 'r-000001', taskId: 'task-2', nodeId: 'review',
  taskDefinition: 'review-docs', agent: 'reviewer', execution: 'subagent',
  request: '', taskInstructions: 'Review the guide.\n',
  agentInstructions: 'Keep useful sections — do not replace the report.\n',
  skills: [{ id: 'readability', directory: '/project/.unelos/skills/readability',
    content: '---\nname: readability\n---\nSelected body.\n\n' },
    { id: 'second', directory: '/skills/second', content: 'Second body.\n' }],
  model: null, mcps: [], tools: [], outcomes: [],
  inputs: [{ name: 'outline.md', path: '/run/outline/outline.md' }],
  outputs: [{ name: 'review.md', path: '/run/review/review.md' }],
  feedback: { sourceNodeId: 'outline', sourceExecutionId: 'task-1',
    outcome: null, reason: null, artifacts: [] },
  completionInstructions: 'Update existing reports; never edit the ledger.',
  ledgerPath: '/run/ledger.md', controllerInstructions: 'CONTROLLER_PRIVATE_MARKER',
};
const options = { binary: 'unelos', project: '/project', bootstrap: 'Bootstrap', version: 'v2' };

test('executor preserves exact fields/order/bytes without controller metadata or source edits', () => {
  const source = structuredClone(packet);
  const expected = structuredClone(packet);
  delete expected.ledgerPath;
  delete expected.controllerInstructions;
  assert.deepEqual(executorPacket(packet), expected);
  const text = executorText(packet, 'New engineers — agreed brief.\n');
  assert.ok(text.includes(JSON.stringify(expected, null, 2)));
  assert.ok(text.includes('New engineers — agreed brief.\n'));
  assert.ok(!text.includes(packet.ledgerPath));
  assert.ok(!text.includes('CONTROLLER_PRIVATE_MARKER'));
  assert.deepEqual(packet, source);
  assert.throws(() => executorText(packet, ''), { code: 'invalid_packet' });
});

test('parallel selection isolates sibling Skills and retains parent identity', () => {
  const first = { ...packet, key: 'worker-1', parentTaskId: 'task-3', skills: [packet.skills[0]] };
  delete first.taskId; delete first.ledgerPath; delete first.controllerInstructions;
  const second = { ...first, key: 'worker-2', skills: [packet.skills[1]] };
  const parallel = { ...packet, kind: 'parallel', workers: [first, second] };
  assert.deepEqual(executorPacket(parallel, 'worker-1'), first);
  assert.ok(!executorText(parallel, 'Brief', 'worker-1').includes('Second body'));
  assert.throws(() => executorPacket(parallel, 'missing'), { code: 'invalid_packet' });
  for (const kind of ['human-gate', 'finished']) {
    assert.throws(() => executorPacket({ kind }), { code: 'invalid_packet' });
  }
});

test('controller separates main duties and renders gate/finished without executor work', () => {
  const main = controllerText({ packet: { ...packet, execution: 'main' } }, options);
  assert.ok(main.startsWith(CONTEXT_TAG));
  assert.ok(main.includes('Executor context:'));
  assert.ok(main.includes('Controller-only duties:'));
  assert.ok(main.includes(packet.controllerInstructions));
  assert.ok(controllerText({ packet }, options).includes('do not perform inline'));
  const common = { runId: packet.runId, nodeId: 'approve', ledgerPath: packet.ledgerPath,
    controllerInstructions: 'Wait for user' };
  for (const item of [
    { ...common, kind: 'human-gate', activationId: 'gate-2', artifacts: packet.outputs,
      decisions: ['approved', 'rejected'], rejectionReasonRequired: true },
    { ...common, kind: 'finished', remainingWork: false },
  ]) {
    const text = controllerText({ packet: item }, options);
    assert.ok(text.includes(JSON.stringify(item, null, 2)));
    assert.ok(!text.includes('Executor context:'));
  }
  const idle = controllerText({ workflows: ['check-guide'] }, options);
  assert.ok(idle.includes('check-guide'));
  assert.ok(idle.includes('/unelos off'));
});

test('malformed packets fail before rendering or preflight', () => {
  for (const change of [{ skills: null }, { tools: [5] }, { inputs: [{ name: 'x' }] },
    { execution: 'unknown' }, { feedback: {} }, { taskId: null }, { model: 5 }]) {
    assert.throws(() => executorPacket({ ...packet, ...change }), { code: 'invalid_packet' });
    assert.throws(() => controllerText({ packet: { ...packet, ...change } }, options),
      { code: 'invalid_packet' });
  }
  assert.throws(() => controllerText({ packet: { kind: 'unknown' } }, options),
    { code: 'invalid_packet' });
});

test('model references retain slash IDs and requested variants; unavailable settings reject', () => {
  assert.deepEqual(parseModel('provider/family/model#high'),
    { providerID: 'provider', id: 'family/model', variant: 'high' });
  assert.equal(parseModel(null), null);
  for (const value of ['model', 'provider/model#', 'provider/model#high#low', 5]) {
    assert.throws(() => parseModel(value), { code: 'invalid_model' });
  }
  assert.throws(() => validateModel('provider/model#high',
    [{ providerID: 'provider', id: 'model', variants: [] }], 'v1'),
    { code: 'required_model_unavailable' });
  assert.throws(() => validateModel('provider/model',
    [{ providerID: 'provider', id: 'model', enabled: false }], 'v2'),
    { code: 'required_model_unavailable' });
  const evidence = { version: 'v2', models: [{ providerID: 'provider', id: 'model', variants: ['high', 'low'] }],
    effectiveModel: { providerID: 'provider', id: 'model', variant: 'high' }, toolIds: new Set() };
  requireConfiguration({ ...packet, model: 'provider/model' }, evidence);
  assert.throws(() => requireConfiguration({ ...packet, model: 'provider/model#low' }, evidence),
    { code: 'model_mismatch' });
  assert.throws(() => requireConfiguration({ ...packet, model: 'provider/model' },
    { ...evidence, effectiveModel: { providerID: 'other', id: 'model' } }), { code: 'model_mismatch' });
});

test('required tools use executor evidence without restricting unrelated tools; MCP blocks', () => {
  const evidence = { version: 'v2', models: [], effectiveModel: null,
    toolIds: new Set(['read', 'shell', 'grep']) };
  requireConfiguration({ ...packet, tools: ['read-file', 'run-command'] }, evidence);
  requireConfiguration({ ...packet, tools: ['run-command'] },
    { ...evidence, version: 'v1', toolIds: new Set(['bash']) });
  assert.throws(() => requireConfiguration({ ...packet, tools: ['read-file'] },
    { ...evidence, toolIds: new Set(['grep']) }), { code: 'required_tool_unavailable' });
  assert.throws(() => requireConfiguration({ ...packet, tools: ['read-file'] },
    { ...evidence, version: 'v1', toolIds: null }), { code: 'effective_tools_unverified' });
  assert.throws(() => requireConfiguration({ ...packet, mcps: ['github'] }, evidence),
    { code: 'unsupported_mcp' });
  assert.deepEqual([...evidence.toolIds], ['read', 'shell', 'grep']);
});
