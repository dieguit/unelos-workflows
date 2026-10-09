import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// Full acceptance only. Partial native smoke checks must never satisfy this gate.
export function verifyExecutionEvidence({inspection,timeline,snapshots,native,recovery,interactive}) {
  assert.equal(inspection.nodes.length,6);
  assert.equal(inspection.edges.length,7);
  const node=id=>inspection.nodes.find(n=>n.id===id);
  assert.equal(node('interview').execution,'main');
  assert.equal(node('plan').execution,'main');
  assert.equal(node('implement').execution,'subagent');
  assert.equal(node('review').execution,'subagent');
  assert.deepEqual(node('review').next,{passed:'finish',changes_requested:'implement'});
  assert.deepEqual(node('approve-plan').next,{approved:'implement',rejected:'plan'});
  assert.ok(Array.isArray(timeline));
  const at=event=>{const rows=timeline.filter(r=>r.event===event);assert.equal(rows.length,1,event);return rows[0];};
  const question=at('worker-question'),answer=at('worker-answer'),resumed=at('worker-resumed');
  assert.deepEqual(question.identity,answer.identity);
  assert.deepEqual(question.identity,resumed.identity);
  assert.equal(question.nativeHandle,resumed.nativeHandle);
  assert.ok(question.question.trim());assert.ok(answer.answer.trim());
  for (const event of ['plan-rejected','plan-approved','worker-answer']) {
    const row=at(event);
    assert.equal(row.source,'explicit-operator-response');
    const message=native.userMessages.find(m=>m.id===row.messageID);
    assert.ok(message?.text?.trim(),event+' native user message');
    assert.equal(message.text,row.operatorText);
  }
  assert.notEqual(at('plan-original').identity.taskId,at('plan-reentered').identity.taskId);
  assert.notEqual(at('implementation-original').identity.taskId,at('implementation-reentered').identity.taskId);
  assert.notEqual(at('implementation-original').nativeHandle,at('review-original').nativeHandle);
  assert.equal(at('finished').identity.kind,'finished');
  for (const prefix of ['plan','implementation']) {
    const first=snapshots[prefix+'-original'],later=snapshots[prefix+'-reentered'];
    assert.deepEqual(first.paths,later.paths);
    assert.ok(first.preservedSection.trim());
    assert.ok(later.report.includes(first.preservedSection));
    assert.ok(later.feedback.trim());assert.ok(later.report.includes(later.feedback));
  }
  assert.ok(snapshots['implementation-reentered'].report.includes(answer.answer));
  assert.ok(/^## Brief\s*\n\S/m.test(snapshots.ledger));
  assert.ok(snapshots.ledger.includes(at('plan-rejected').operatorText));
  const tasks=['implementation-original','review-original','implementation-reentered'];
  for (const name of tasks) {
    const row=at(name),worker=native.workers.find(w=>w.handle===row.nativeHandle && w.taskId===row.identity.taskId);
    assert.ok(worker,name+' worker evidence');
    assert.equal(worker.deliveredExecutorText,worker.expectedExecutorText);
    assert.ok(!worker.deliveredExecutorText.includes(worker.ledgerPath));
    assert.ok(!worker.deliveredExecutorText.includes(worker.controllerInstructions));
    assert.deepEqual(worker.actualModel,worker.selectedModel);
    assert.ok(worker.requiredTools.every(tool=>worker.actualTools.includes(tool)));
    assert.ok(!worker.writes.includes(worker.ledgerPath));
  }
  const q=timeline.indexOf(question),a=timeline.indexOf(answer);
  assert.ok(q<a);
  assert.ok(!timeline.slice(q+1,a).some(row=>row.event==='mutation'));
  assert.ok(native.helperCalls.length>0);
  for (const call of native.helperCalls) {
    assert.equal(call.tool,'shell');assert.equal(call.receipt.ok,true);
    assert.deepEqual(call.expected,call.freshIdentity);
    assert.ok(call.time<call.finalResponseTime);
  }
  assert.deepEqual(recovery.before,recovery.after);
  assert.ok(!recovery.operations.some(op=>['dispatch','complete','approve','reject','write'].includes(op)));
  assert.ok(interactive.version && interactive.interface && interactive.source);
  assert.equal(interactive.blocker,null);
  assert.ok(interactive.userMessageIDs.length>0);
  for (const id of interactive.userMessageIDs) assert.ok(native.userMessages.some(m=>m.id===id));
  return true;
}

if (process.argv[1] && resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const root=process.argv[2];assert.ok(root,'Pass a redacted T16 evidence directory');
  const evidence=Object.fromEntries(['inspection','timeline','snapshots','native','recovery','interactive'].map(name=>[name,JSON.parse(readFileSync(join(root,name+'.json'),'utf8'))]));
  verifyExecutionEvidence(evidence);
  console.log('Full T16 execution evidence checks passed');
}
