import test from 'node:test';
import assert from 'node:assert/strict';
import { verifyExecutionEvidence } from './execution-evidence.mjs';

test('execution acceptance rejects absent evidence rather than trusting passed flags', () => {
  for (const evidence of [{}, { passed:true }, { timeline:[], native:{passed:true} }]) {
    assert.throws(()=>verifyExecutionEvidence(evidence));
  }
});

test('synthetic checker fixture rejects changed Task continuation and missing interactive proof', () => {
  // This fixture tests the checker only; it is never native acceptance evidence.
  const id={kind:'task',runId:'r-1',nodeId:'implement',taskId:'task-3'};
  const timeline=[
    {event:'worker-question',identity:id,nativeHandle:'worker-1',question:'Where?'},
    {event:'worker-answer',identity:id,answer:'Keep mobile',source:'explicit-operator-response',messageID:'msg-answer',operatorText:'Keep mobile'},
    {event:'worker-resumed',identity:id,nativeHandle:'worker-1'},
    {event:'plan-rejected',source:'explicit-operator-response',messageID:'msg-reject',operatorText:'Add check'},
    {event:'plan-approved',source:'explicit-operator-response',messageID:'msg-approve',operatorText:'Approve revision'},
    {event:'plan-original',identity:{taskId:'task-2'}}, {event:'plan-reentered',identity:{taskId:'task-4'}},
    {event:'implementation-original',identity:id,nativeHandle:'worker-1'},
    {event:'implementation-reentered',identity:{...id,taskId:'task-6'},nativeHandle:'worker-3'},
    {event:'review-original',identity:{taskId:'task-5'},nativeHandle:'worker-2'},
    {event:'finished',identity:{kind:'finished'}},
  ];
  const evidence={inspection:{nodes:[{id:'interview',execution:'main'},{id:'plan',execution:'main'},
    {id:'implement',execution:'subagent'},{id:'review',execution:'subagent',next:{passed:'finish',changes_requested:'implement'}},
    {id:'approve-plan',next:{approved:'implement',rejected:'plan'}},{id:'finish'}],edges:Array(7).fill({})},timeline,
    snapshots:{ledger:'## Brief\nChange heading\nAdd check'},
    native:{userMessages:[{id:'msg-answer',text:'Keep mobile'},{id:'msg-reject',text:'Add check'},{id:'msg-approve',text:'Approve revision'}],workers:[],helperCalls:[{tool:'shell',receipt:{ok:true},expected:id,freshIdentity:id,time:1,finalResponseTime:2}]},
    recovery:{before:{hash:'a'},after:{hash:'a'},operations:['status','next']},
    interactive:{version:'test-fixture',interface:'fixture',source:'synthetic-checker-test',blocker:null,userMessageIDs:['msg-answer']}};
  for (const prefix of ['plan','implementation']) {
    evidence.snapshots[prefix+'-original']={paths:['/report'],preservedSection:'Useful section'};
    evidence.snapshots[prefix+'-reentered']={paths:['/report'],report:'Useful section\nCorrection\nKeep mobile',feedback:'Correction'};
  }
  for (const [handle,taskId] of [['worker-1','task-3'],['worker-2','task-5'],['worker-3','task-6']]) {
    evidence.native.workers.push({handle,taskId,deliveredExecutorText:'Exact packet',expectedExecutorText:'Exact packet',ledgerPath:'/private/ledger',controllerInstructions:'Private controller',actualModel:{id:'model'},selectedModel:{id:'model'},requiredTools:['read'],actualTools:['read'],writes:['/report']});
  }
  assert.equal(verifyExecutionEvidence(evidence),true);
  for (const change of [e=>e.timeline[2].identity={...id,taskId:'later'},e=>e.interactive.blocker='No TUI',
    e=>e.native.workers[0].writes.push('/private/ledger'),e=>e.native.userMessages[0].text='Different answer',
    e=>e.native.helperCalls[0].freshIdentity={...id,taskId:'later'}]) {
    const bad=structuredClone(evidence);change(bad);assert.throws(()=>verifyExecutionEvidence(bad));
  }
});
