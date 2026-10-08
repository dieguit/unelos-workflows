export const CONTEXT_TAG = '<unelos-opencode-context>\n';

function fail(code, message) {
  throw Object.assign(new Error(message), { code });
}

const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const strings = (value, keys) => object(value) && keys.every(key => typeof value[key] === 'string');
const stringArray = value => Array.isArray(value) && value.every(item => typeof item === 'string');
const artifacts = value => Array.isArray(value) && value.every(item => strings(item, ['name', 'path']));

function contextValid(packet) {
  return strings(packet, ['nodeId', 'taskDefinition', 'agent', 'taskInstructions',
    'agentInstructions', 'completionInstructions', 'runId', 'request'])
    && packet.kind === 'task' && ['main', 'subagent'].includes(packet.execution)
    && (packet.model === null || typeof packet.model === 'string')
    && ['mcps', 'tools', 'outcomes'].every(key => stringArray(packet[key]))
    && Array.isArray(packet.skills)
    && packet.skills.every(skill => strings(skill, ['id', 'directory', 'content']))
    && artifacts(packet.inputs) && artifacts(packet.outputs)
    && (packet.feedback === null || (strings(packet.feedback, ['sourceNodeId', 'sourceExecutionId'])
      && ['outcome', 'reason'].every(key => packet.feedback[key] === null
        || typeof packet.feedback[key] === 'string') && artifacts(packet.feedback.artifacts)));
}

export function validatePacket(packet) {
  let valid = strings(packet, ['runId', 'nodeId']);
  if (valid && packet.kind === 'task') {
    valid = contextValid(packet) && (typeof packet.taskId === 'string'
      || strings(packet, ['parentTaskId', 'key']));
  } else if (valid && packet.kind === 'parallel') {
    valid = strings(packet, ['taskId', 'request', 'ledgerPath', 'controllerInstructions'])
      && artifacts(packet.inputs) && artifacts(packet.outputs)
      && Array.isArray(packet.workers) && packet.workers.every(worker => contextValid(worker)
        && strings(worker, ['parentTaskId', 'key']));
  } else if (valid && packet.kind === 'human-gate') {
    valid = strings(packet, ['activationId', 'ledgerPath', 'controllerInstructions'])
      && artifacts(packet.artifacts) && stringArray(packet.decisions)
      && typeof packet.rejectionReasonRequired === 'boolean';
  } else if (valid && packet.kind === 'finished') {
    valid = strings(packet, ['ledgerPath', 'controllerInstructions'])
      && typeof packet.remainingWork === 'boolean';
  } else valid = false;
  if (!valid) fail('invalid_packet', 'Invalid Unelos packet fields or kind');
  return packet;
}

export function executorPacket(packet, workerKey) {
  validatePacket(packet);
  let selected = packet;
  if (packet.kind === 'parallel') {
    selected = packet.workers.find(worker => worker.key === workerKey);
    if (!selected) fail('invalid_packet', 'Unknown parallel worker key');
  } else if (packet.kind !== 'task' || workerKey !== undefined) {
    fail('invalid_packet', 'Executor context requires one Task or parallel worker');
  }
  const { ledgerPath, controllerInstructions, ...executor } = selected;
  return executor;
}

export function executorText(packet, brief, workerKey) {
  if (typeof brief !== 'string' || !brief.trim()) {
    fail('invalid_packet', 'A controller-supplied brief is required before worker work');
  }
  return [
    'Supplied Skills are already loaded. Do not reload or substitute them.',
    'Read bound inputs, feedback artifacts, and existing outputs before work.',
    'Preserve useful content and update the same report paths in place.',
    'Report changed paths/progress to the controller; never edit the ledger or advance Unelos.',
    'If essential context is missing, return the question without claiming completion.',
    'Relevant controller-supplied brief:', brief,
    'Exact Unelos executor packet:', JSON.stringify(executorPacket(packet, workerKey), null, 2),
  ].join('\n\n');
}

export function workflowChoices(workflows) {
  if (!stringArray(workflows)) fail('invalid_packet', 'Invalid Workflow choices');
  return ['Present this numbered list; wait for explicit selection even if there is only one Workflow:',
    ...workflows.map((name, index) => `${index + 1}. ${name}`),
    `${workflows.length + 1}. Continue without Unelos (/unelos off)`,
    'Reply with a number or Workflow name. Map the reply to this displayed list; ask if ambiguous. Never auto-start.'].join('\n');
}

export function controllerText(current, { binary, project, bootstrap, version }) {
  const header = [CONTEXT_TAG, bootstrap,
    `CLI executable: ${JSON.stringify(binary)}; project: ${JSON.stringify(project)}`,
    `Native read: read; command tool: ${version === 'v1' ? 'bash' : 'shell'}`,
    version === 'v1'
      ? 'Native worker tool: task(subagent_type, description, prompt, task_id for continuation).'
      : 'Native worker tool: subagent(agent, description, prompt, model, sessionID for continuation).'];
  if (!current.packet) {
    return [...header, workflowChoices(current.workflows)].join('\n\n');
  }
  const packet = validatePacket(current.packet);
  if (!strings(packet, ['ledgerPath', 'controllerInstructions'])) {
    fail('invalid_packet', 'Missing controller metadata');
  }
  if (packet.kind === 'task' && packet.execution === 'main') {
    return [...header, 'Executor context:', JSON.stringify(executorPacket(packet), null, 2),
      'Controller-only duties:', JSON.stringify({ ledgerPath: packet.ledgerPath,
        controllerInstructions: packet.controllerInstructions }, null, 2)].join('\n\n');
  }
  return [...header, ['task', 'parallel'].includes(packet.kind)
    ? 'Delegated work: do not perform inline.' : 'Controller context only:',
  JSON.stringify(packet, null, 2)].join('\n\n');
}

export function parseModel(reference) {
  if (reference === null || reference === undefined) return null;
  const match = typeof reference === 'string'
    && /^([^/\s#]+)\/([^\s#]+)(?:#([^\s#]+))?$/.exec(reference);
  if (!match) fail('invalid_model', 'Invalid required model reference');
  return { providerID: match[1], id: match[2], ...(match[3] ? { variant: match[3] } : {}) };
}

export function validateModel(reference, models, version) {
  const wanted = parseModel(reference);
  if (!wanted) return null;
  const model = models.find(item => item.providerID === wanted.providerID
    && item.id === wanted.id && item.enabled !== false);
  if (!model || (wanted.variant && !model.variants?.includes(wanted.variant))) {
    fail('required_model_unavailable', `Unelos (${version}): required model/variant unavailable: ${reference}`);
  }
  return wanted;
}

export function requireConfiguration(packet, { version, models, effectiveModel, toolIds }) {
  validatePacket(packet);
  if (packet.kind !== 'task') fail('invalid_packet', 'Preflight requires one executor Task');
  if (packet.mcps.length) {
    fail('unsupported_mcp', `Unelos (${version}): unsupported MCP requirements: ${packet.mcps.join(', ')}`);
  }
  const wanted = validateModel(packet.model, models, version);
  if (wanted && (wanted.providerID !== effectiveModel?.providerID || wanted.id !== effectiveModel?.id
    || (wanted.variant !== undefined && wanted.variant !== effectiveModel?.variant))) {
    fail('model_mismatch', `Unelos (${version}): effective model mismatch for ${packet.model}; submit a fresh prompt`);
  }
  for (const reference of packet.tools) {
    const name = reference === 'read-file' ? 'read'
      : reference === 'run-command' ? (version === 'v1' ? 'bash' : 'shell') : reference;
    if (toolIds === null) fail('effective_tools_unverified',
      `Unelos (${version}): cannot establish effective required tool: ${reference} (unverified)`);
    if (!toolIds.has(name)) fail('required_tool_unavailable',
      `Unelos (${version}): required tool unavailable: ${reference} -> ${name}`);
  }
}
