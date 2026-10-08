import { readFile, realpath } from 'node:fs/promises';
import { cliJson, readCurrent } from './cli.js';
import { CONTEXT_TAG, controllerText, validatePacket,
  validateModel, requireConfiguration } from './packet.js';

const OWNER = 'unelos.opencode';
const owned = part => part.type === 'text' && part.text.startsWith(CONTEXT_TAG);
const error = (code, message) => Object.assign(new Error(message), { code });

function binaryOption(options) {
  const binary = options?.binary ?? 'unelos';
  if (typeof binary !== 'string' || !binary.trim() || binary.includes('\0')) {
    throw error('invalid_binary', 'Unelos binary must be one executable path or PATH name, not a shell command');
  }
  return binary;
}

function checkSession(session, sessionID) {
  if (!session || session.id !== sessionID) throw error('session_lookup_failed', 'Unelos native session lookup failed');
  return session;
}

async function activeProject(session, directory) {
  if (session.parentID != null) return null;
  const active = session.metadata?.[OWNER];
  if (active === undefined) return null;
  if (!active || typeof active.project !== 'string' || typeof active.enabled !== 'boolean') {
    throw error('invalid_activation', 'Invalid Unelos activation metadata');
  }
  if (!active.enabled) return null;
  const project = await realpath(directory);
  if (active.project !== project) throw error('project_changed', 'Unelos project changed; reactivate explicitly');
  return project;
}

function argumentsFor(text) {
  const words = text.trim().split(/\s+/).filter(Boolean);
  if ((words[0] === 'off' && words.length === 1) || !words.length
    || (words[0] === 'start' && words.length <= 2)) return words;
  throw error('invalid_command', 'Usage: /unelos [start [WORKFLOW] | off]');
}

async function commandGuidance(binary, project, words) {
  if (words[0] === 'off') return 'Continue without Unelos. Do not issue Unelos progression commands in this session.';
  const current = await readCurrent(binary, project);
  if (['running', 'waiting'].includes(current.status.status)) {
    return `Continue the existing run ${JSON.stringify(current.status.runId)}; never replace it.`;
  }
  if (words[1]) return `Workflow explicitly selected: ${JSON.stringify(words[1])}. Obtain the agreed goal, supply a descriptive name at start, and write the ledger brief before work. Core validates the Workflow.`;
  if (current.status.status === 'finished' && !words.length) {
    return 'The current run is finished. Report that result; do not start another run.';
  }
  const workflows = current.workflows ?? (await cliJson(binary, project, 'start')).workflows;
  return `Offer Workflow choices ${JSON.stringify(workflows)} and Continue without Unelos (/unelos off); wait for selection even if there is only one choice.`;
}

function metadataFor(session, project, off) {
  if (session.parentID != null) throw error('child_controller', 'A child session cannot activate an Unelos controller');
  const metadata = { ...session.metadata };
  if (off) delete metadata[OWNER];
  else metadata[OWNER] = { project, enabled: true };
  return metadata;
}

// Static reference checks do not mistake parent tools/model for a delegated executor's.
async function referenceChecks(packet, version, models) {
  if (!packet) return;
  validatePacket(packet);
  const tasks = packet.kind === 'parallel' ? packet.workers : packet.kind === 'task' ? [packet] : [];
  for (const task of tasks) {
    if (task.mcps.length) throw error('unsupported_mcp',
      `Unelos (${version}): unsupported MCP requirements: ${task.mcps.join(', ')}`);
    if (task.model !== null) validateModel(task.model, await models(), version);
    if (version === 'v1' && task.tools.length) throw error('effective_tools_unverified',
      `Unelos (v1): cannot establish effective required tools: ${task.tools.join(', ')} (unverified)`);
  }
}

export default {
  id: OWNER,
  async setup(ctx) {
    const binary = binaryOption(ctx.options);
    const bootstrap = await readFile(new URL('./bootstrap.md', import.meta.url), 'utf8');
    const models = async project => (await ctx.model.list({ location: { directory: project } }))
      .map(item => ({ providerID: item.providerID, id: item.id, enabled: item.enabled,
        variants: Array.isArray(item.variants) ? item.variants.map(variant => variant.id) : [] }));
    const get = async sessionID => checkSession(await ctx.session.get({ sessionID }), sessionID);
    await ctx.command.transform(editor => editor.add({
      name: 'unelos', description: 'Select or continue an explicitly activated Unelos workflow',
      execute: async ({ sessionID, prompt, delivery }) => {
        const words = argumentsFor(prompt.text);
        const session = await get(sessionID);
        if (session.parentID != null) throw error('child_controller', 'A child session cannot activate an Unelos controller');
        const project = await realpath(session.location.directory);
        const text = await commandGuidance(binary, project, words);
        await ctx.session.update({ sessionID, metadata: metadataFor(session, project, words[0] === 'off') });
        await ctx.session.prompt({ ...prompt, sessionID, delivery, text: text + '\n\n' + prompt.text });
      },
    }));
    await ctx.session.hook('prompt', async event => {
      const session = await get(event.sessionID);
      const project = await activeProject(session, session.location.directory);
      if (!project) return;
      const current = await readCurrent(binary, project);
      await referenceChecks(current.packet, 'v2', () => models(project));
      if (current.packet?.kind !== 'task' || current.packet.execution !== 'main') return;
      const model = validateModel(current.packet.model, current.packet.model ? await models(project) : [], 'v2');
      if (model) await ctx.session.switchModel({ sessionID: event.sessionID, model });
    });
    await ctx.session.hook('context', async event => {
      event.system = event.system.filter(part => !owned(part));
      const session = await get(event.sessionID);
      const project = await activeProject(session, session.location.directory);
      if (!project) return;
      const current = await readCurrent(binary, project);
      await referenceChecks(current.packet, 'v2', () => models(project));
      if (current.packet?.kind === 'task' && current.packet.execution === 'main') {
        requireConfiguration(current.packet, { version: 'v2',
          models: current.packet.model ? await models(project) : [],
          effectiveModel: event.model, toolIds: new Set(Object.keys(event.tools)) });
      }
      event.system.push({ type: 'text', text: controllerText(current, { binary, project, bootstrap, version: 'v2' }) });
    });
  },
  async server({ client }, options) {
    const binary = binaryOption(options);
    const bootstrap = await readFile(new URL('./bootstrap.md', import.meta.url), 'utf8');
    const get = async sessionID => {
      const result = await client.session.get({ path: { id: sessionID } });
      if (result.error || result.response?.ok === false) throw error('session_lookup_failed', 'Unelos native session lookup failed');
      return checkSession(result.data, sessionID);
    };
    const models = async project => {
      const result = await client.config.providers({ query: { directory: project } });
      if (result.error || result.response?.ok === false || !Array.isArray(result.data?.providers)) {
        throw error('model_catalogue_unavailable', 'Unelos (v1): native model catalogue unavailable');
      }
      return result.data.providers.flatMap(provider => Object.values(provider.models).map(item => ({
        providerID: provider.id, id: item.id, enabled: true,
        variants: item.variants && typeof item.variants === 'object' ? Object.keys(item.variants) : [],
      })));
    };
    const currentFor = async sessionID => {
      const session = await get(sessionID);
      const project = await activeProject(session, session.directory);
      if (!project) return null;
      const current = await readCurrent(binary, project);
      await referenceChecks(current.packet, 'v1', () => models(project));
      return { project, current };
    };
    return {
      'command.execute.before': async (input, output) => {
        if (input.command !== 'unelos') return;
        const words = argumentsFor(input.arguments);
        const session = await get(input.sessionID);
        if (session.parentID != null) throw error('child_controller', 'A child session cannot activate an Unelos controller');
        const project = await realpath(session.directory);
        const text = await commandGuidance(binary, project, words);
        const result = await client.session.update({ path: { id: input.sessionID }, body: {
          metadata: metadataFor(session, project, words[0] === 'off'),
        } });
        if (result.error || result.response?.ok === false) throw error('activation_failed', 'Unelos activation could not be persisted');
        const part = output.parts.find(part => part.type === 'text');
        if (!part) throw error('delivery_text_missing', 'Unelos command requires native text');
        part.text = text;
      },
      'chat.message': async (input, output) => {
        const state = await currentFor(input.sessionID);
        const packet = state?.current.packet;
        if (packet?.kind !== 'task' || packet.execution !== 'main') return;
        const model = validateModel(packet.model, packet.model ? await models(state.project) : [], 'v1');
        if (model) output.message.model = { providerID: model.providerID, modelID: model.id,
          ...(model.variant ? { variant: model.variant } : {}) };
      },
      'chat.params': async input => {
        const state = await currentFor(input.sessionID);
        const packet = state?.current.packet;
        if (packet?.kind !== 'task' || packet.execution !== 'main') return;
        requireConfiguration(packet, { version: 'v1', models: packet.model ? await models(state.project) : [],
          effectiveModel: { providerID: input.model.providerID, id: input.model.id,
            ...(input.message.model?.variant ? { variant: input.message.model.variant } : {}) }, toolIds: null });
      },
      'experimental.chat.messages.transform': async (_input, output) => {
        for (const message of output.messages) message.parts = message.parts.filter(part => !owned(part));
        const user = output.messages.find(message => message.info.role === 'user');
        if (!user) return;
        const state = await currentFor(user.info.sessionID);
        if (!state) return;
        const target = output.messages.find(message => message.info.role === 'user'
          && message.parts.some(part => part.type === 'text' && part.text.trim()));
        if (!target) throw error('delivery_text_missing', 'Unelos (v1): delivery needs ordinary user text after compaction');
        const template = target.parts.find(part => part.type === 'text');
        target.parts.unshift({ ...template, text: controllerText(state.current,
          { binary, project: state.project, bootstrap, version: 'v1' }) });
      },
    };
  },
};
