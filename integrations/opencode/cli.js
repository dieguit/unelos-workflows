import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
const execute = promisify(execFile);

export async function cliJson(binary, project, command) {
  if (!['status', 'next', 'start'].includes(command)) {
    throw new Error('Integration CLI reads cannot invoke mutations');
  }
  let stdout;
  try {
    ({ stdout } = await execute(binary, ['--project', project, command, '--json'],
      { encoding: 'utf8', timeout: 10000, maxBuffer: 16 * 1024 * 1024 }));
  } catch (error) {
    let diagnostic;
    for (const line of String(error.stderr ?? '').trim().split('\n').reverse()) {
      try {
        const value = JSON.parse(line)?.error;
        if (typeof value?.message === 'string' && typeof value?.code === 'string') {
          diagnostic = value; break;
        }
      } catch { /* Non-JSON stderr is not safe to expose. */ }
    }
    throw Object.assign(new Error(diagnostic?.message ?? `Unelos ${command} failed`),
      { code: diagnostic?.code ?? 'cli_failed' });
  }
  let value;
  try { value = JSON.parse(stdout); } catch {
    throw Object.assign(new Error(`Unelos ${command} returned invalid JSON`), { code: 'invalid_cli_json' });
  }
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw Object.assign(new Error(`Unelos ${command} returned no object`), { code: 'invalid_cli_json' });
  }
  return value;
}

export async function readCurrent(binary, project) {
  const status = await cliJson(binary, project, 'status');
  if (status.status === 'idle') {
    const selection = await cliJson(binary, project, 'start');
    if (!Array.isArray(selection.workflows) || !selection.workflows.every(item => typeof item === 'string')) {
      throw Object.assign(new Error('Unelos returned invalid Workflow choices'), { code: 'invalid_cli_status' });
    }
    return { status, workflows: selection.workflows };
  }
  if (!['running', 'waiting', 'finished'].includes(status.status)) {
    throw Object.assign(new Error('Unelos returned an unknown status'), { code: 'invalid_cli_status' });
  }
  const packet = await cliJson(binary, project, 'next');
  if (typeof status.runId !== 'string' || status.runId !== packet.runId
    || status.current?.nodeId !== packet.nodeId || status.current?.kind !== packet.kind) {
    throw Object.assign(new Error('Unelos current work changed between status and next; read again'),
      { code: 'invalid_cli_status' });
  }
  return { status, packet };
}
