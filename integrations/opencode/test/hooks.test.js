import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, chmodSync, rmSync, existsSync, readFileSync } from 'node:fs';
import plugin from '../index.js';
import { CONTEXT_TAG } from '../packet.js';

const task = { kind: 'task', runId: 'r-000001', taskId: 'task-1', nodeId: 'outline',
  taskDefinition: 'outline', agent: 'writer', execution: 'main', request: '',
  taskInstructions: 'Write outline', agentInstructions: 'New engineers',
  skills: [{ id: 'outline', directory: '/skills/outline', content: 'SELECTED\n\n' }],
  model: null, tools: [], mcps: [], outcomes: [], inputs: [], outputs: [], feedback: null,
  completionInstructions: 'Preserve report', ledgerPath: '/run/ledger.md', controllerInstructions: 'CONTROLLER_PRIVATE' };

async function fixture(t, version) {
  const project = mkdtempSync('/tmp/opencode/t15-hooks-');
  t.after(() => rmSync(project, { recursive: true }));
  const binary = project + '/cli.cjs';
  writeFileSync(binary, `#!/usr/bin/env node
const fs=require('node:fs'), project=process.argv[3], command=process.argv[4];
fs.appendFileSync(project+'/calls',command+'\\n');
console.log(fs.readFileSync(project+'/'+command+'.json','utf8'));
`);
  chmodSync(binary, 0o755);
  let session = { id: 'ses_controller', directory: project, location: { directory: project },
    metadata: { other: { keep: true } }, model: { providerID: 'provider', id: 'model' } };
  const hooks = new Map(), commands = new Map(), prompts = [], switches = [];
  const models = [{ providerID: 'provider', id: 'model', variants: [] },
    { providerID: 'provider', id: 'required', variants: [] }];
  const f = { project, binary, hooks, prompts, switches, session,
    set(packet) {
      writeFileSync(project + '/status.json', JSON.stringify(packet ? {
        status: packet.kind === 'human-gate' ? 'waiting' : packet.kind === 'finished' ? 'finished' : 'running',
        runId: packet.runId, current: { kind: packet.kind, nodeId: packet.nodeId },
      } : { status: 'idle' }));
      writeFileSync(project + '/next.json', JSON.stringify(packet));
      writeFileSync(project + '/start.json', JSON.stringify({ workflows: ['guide'] }));
    },
    calls() { return existsSync(project + '/calls') ? readFileSync(project + '/calls', 'utf8').trim().split('\n') : []; },
    lookupError: false, lookupMismatch: false,
  };
  f.set(null);
  const get = () => {
    if (f.lookupError) throw new Error('Native lookup failed');
    return structuredClone({ ...session, id: f.lookupMismatch ? 'ses_wrong' : session.id });
  };
  if (version === 'v2') {
    const ctx = { options: { binary },
      session: {
        get: async ({ sessionID }) => { assert.equal(sessionID, session.id); return get(); },
        update: async ({ sessionID, metadata }) => { assert.equal(sessionID, session.id); session.metadata = metadata; },
        prompt: async value => { prompts.push(value); },
        switchModel: async ({ sessionID, model }) => { assert.equal(sessionID, session.id); switches.push(model); session.model = model; },
        hook: async (name, fn) => { hooks.set(name, fn); },
      },
      command: { transform: async fn => fn({ add: command => commands.set(command.name, command) }) },
      model: { list: async ({ location }) => { assert.equal(location.directory, project); return models; } },
    };
    f.restart = () => plugin.setup(ctx);
    await f.restart();
    f.command = text => commands.get('unelos').execute({ sessionID: session.id, prompt: { text, files: [] }, delivery: 'steer' });
    f.event = () => ({ sessionID: session.id, model: session.model, system: [{ type: 'text', text: 'Other plugin' }],
      tools: { read: {}, shell: {}, grep: {} }, messages: [] });
    f.context = event => hooks.get('context')(event);
    f.blocks = event => event.system.filter(part => part.text.startsWith(CONTEXT_TAG));
    f.admit = event => hooks.get('prompt')(event);
  } else {
    const client = { session: {
      get: async ({ path }) => { assert.equal(path.id, session.id); return { data: get(), response: { ok: true } }; },
      update: async ({ path, body }) => { assert.equal(path.id, session.id); session.metadata = body.metadata; return { data: get(), response: { ok: true } }; },
    }, config: { providers: async ({ query }) => {
      assert.equal(query.directory, project);
      return { data: { providers: [{ id: 'provider', models: Object.fromEntries(models.map(item => [item.id, item])) }] } };
    } } };
    f.restart = async () => { f.native = await plugin.server({ client }, { binary }); };
    await f.restart();
    f.command = async text => {
      const output = { parts: [{ type: 'text', text: 'Command template' }, { type: 'file', url: 'file:///keep' }] };
      await f.native['command.execute.before']({ command: 'unelos', sessionID: session.id, arguments: text }, output);
      prompts.push(output); return output;
    };
    f.event = () => ({ messages: [
      { info: { role: 'user', sessionID: session.id }, parts: [{ type: 'compaction' }] },
      { info: { role: 'user', sessionID: session.id }, parts: [{ type: 'text', text: 'Continue' }] },
    ] });
    f.context = event => f.native['experimental.chat.messages.transform']({}, event);
    f.blocks = event => event.messages.flatMap(message => message.parts).filter(part => part.text?.startsWith(CONTEXT_TAG));
    f.admit = output => f.native['chat.message']({ sessionID: session.id }, output);
    f.params = input => f.native['chat.params']({ sessionID: session.id, ...input }, {});
  }
  return f;
}

for (const version of ['v1', 'v2']) {
  test(`${version}: ordinary marker text never activates; native selection/off preserves unrelated metadata`, async t => {
    const f = await fixture(t, version), event = f.event();
    await f.context(event); assert.equal(f.blocks(event).length, 0); assert.deepEqual(f.calls(), []);
    await f.command(''); assert.equal(f.session.metadata['unelos.opencode'].project, f.project);
    assert.deepEqual(f.session.metadata.other, { keep: true });
    assert.ok(JSON.stringify(f.prompts).includes('wait for selection'));
    const count = f.calls().length;
    await f.command('off'); assert.equal(f.session.metadata['unelos.opencode'], undefined);
    assert.equal(f.calls().length, count);
    await f.context(event); assert.equal(f.blocks(event).length, 0);
    assert.deepEqual(f.session.metadata.other, { keep: true });
  });
  test(`${version}: repeated callbacks replace owned content; changed Skill and restart recover same identity`, async t => {
    const f = await fixture(t, version); f.set(task); await f.command('');
    const event = f.event(); await f.context(event); await f.context(event);
    assert.equal(f.blocks(event).length, 1); assert.ok(f.blocks(event)[0].text.includes('SELECTED'));
    const changed = { ...task, skills: [{ ...task.skills[0], content: 'CHANGED\n' }] };
    f.set(changed); await f.context(event);
    assert.ok(!f.blocks(event)[0].text.includes('SELECTED')); assert.ok(f.blocks(event)[0].text.includes('CHANGED'));
    await f.restart(); await f.context(event); assert.equal(f.blocks(event).length, 1);
    assert.ok(JSON.stringify(event).includes(version === 'v2' ? 'Other plugin' : 'Continue'));
    assert.ok(f.calls().every(command => ['status', 'next'].includes(command)));
    assert.deepEqual(f.switches, []);
  });
  test(`${version}: child sessions excluded and lookup/move errors stop instead of stale delivery`, async t => {
    const f = await fixture(t, version); f.set(task); await f.command('');
    f.session.parentID = 'ses_parent'; const event = f.event(); const count = f.calls().length;
    await f.context(event); assert.equal(f.blocks(event).length, 0); assert.equal(f.calls().length, count);
    await assert.rejects(f.command(''), /child|controller/i);
    delete f.session.parentID;
    f.lookupError = true; await assert.rejects(f.context(event), /lookup/i); f.lookupError = false;
    f.lookupMismatch = true; await assert.rejects(f.context(event), /session|lookup/i); f.lookupMismatch = false;
    f.session.metadata['unelos.opencode'].project = '/other';
    await assert.rejects(f.context(event), /project.*changed/i);
    f.session.metadata['unelos.opencode'] = {};
    await assert.rejects(f.context(event), /metadata/i);
  });
  test(`${version}: command grammar rejects before activation; running/waiting/finished never auto-start`, async t => {
    const f = await fixture(t, version);
    await assert.rejects(f.command('invalid'), /Usage/); assert.equal(f.session.metadata['unelos.opencode'], undefined);
    await f.command('start guide'); assert.ok(JSON.stringify(f.prompts).includes('explicitly selected'));
    for (const item of [task, { kind: 'human-gate', runId: task.runId, nodeId: 'approve', activationId: 'gate-2',
      artifacts: [], decisions: ['approved', 'rejected'], rejectionReasonRequired: true,
      ledgerPath: task.ledgerPath, controllerInstructions: 'Wait' },
      { kind: 'finished', runId: task.runId, nodeId: 'done', remainingWork: false,
        ledgerPath: task.ledgerPath, controllerInstructions: 'Finished' }]) {
      f.set(item); const count = f.calls().length; await f.command(''); await f.context(f.event());
      assert.ok(!f.calls().slice(count).includes('start'));
    }
    await f.command('start'); assert.ok(f.calls().includes('start'));
  });
  test(`${version}: configured main model applied at admission and checked at dispatch`, async t => {
    const f = await fixture(t, version); f.set({ ...task, model: 'provider/required' }); await f.command('');
    if (version === 'v2') {
      await f.admit({ sessionID: f.session.id, prompt: { text: 'Continue' } });
      assert.deepEqual(f.switches, [{ providerID: 'provider', id: 'required' }]);
      await f.context(f.event());
      await assert.rejects(f.context({ ...f.event(), model: { providerID: 'provider', id: 'model' } }), { code: 'model_mismatch' });
    } else {
      const output = { message: { agent: 'build', model: { providerID: 'provider', modelID: 'model' } }, parts: [] };
      await f.admit(output);
      assert.deepEqual(output.message.model, { providerID: 'provider', modelID: 'required' });
      assert.equal(output.message.agent, 'build');
      await f.params({ model: { providerID: 'provider', id: 'required' }, message: output.message });
      await assert.rejects(f.params({ model: { providerID: 'other', id: 'required' }, message: output.message }), { code: 'model_mismatch' });
    }
    f.set({ ...task, model: 'provider/missing' });
    await assert.rejects(f.context(f.event()), { code: 'required_model_unavailable' });
    f.set({ ...task, model: 'provider/model#unknown' });
    await assert.rejects(f.context(f.event()), { code: 'required_model_unavailable' });
  });
  test(`${version}: executor requirements reject MCP and unavailable tools without advancement`, async t => {
    const f = await fixture(t, version); f.set({ ...task, mcps: ['github'] }); await f.command('');
    await assert.rejects(f.context(f.event()), { code: 'unsupported_mcp' });
    f.set({ ...task, tools: ['read-file'] });
    if (version === 'v2') {
      await f.context(f.event());
      await assert.rejects(f.context({ ...f.event(), tools: { grep: {} } }), { code: 'required_tool_unavailable' });
    } else await assert.rejects(f.context(f.event()), { code: 'effective_tools_unverified' });
    assert.ok(f.calls().every(command => ['status', 'next'].includes(command)));
  });
}
test('V1 textless recovery blocks instead of silently dropping controller context', async t => {
  const f = await fixture(t, 'v1'); f.set(task); await f.command('');
  const event = f.event(); event.messages.pop();
  await assert.rejects(f.context(event), { code: 'delivery_text_missing' });
});
test('invalid binary options reject instead of accepting shell commands', async () => {
  await assert.rejects(plugin.setup({ options: { binary: 5 } }), /binary/i);
  await assert.rejects(plugin.server({}, { binary: '' }), /binary/i);
});
