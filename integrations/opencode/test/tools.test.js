import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,chmodSync,existsSync,rmSync} from 'node:fs';
import * as module from '../index.js';
import {setTimeout as delay} from 'node:timers/promises';

const names=['unelos_next','unelos_start','unelos_resume','unelos_complete','unelos_approve','unelos_reject','unelos_continue','unelos_status','unelos_inspect','unelos_rename'];
const expected={kind:'task',runId:'r-1',nodeId:'implement',taskId:'task-3'};
const pkg={runId:'r-1',status:'running',workflows:[],expected,packet:{agent:'engineer'},workerPrompt:'Exact core prompt\nSELECTED BODY',controllerInstructions:'Shared controller policy'};

async function fixture(t,version) {
  const project=mkdtempSync('/tmp/opencode/unelos-thin-tools-');t.after(()=>rmSync(project,{recursive:true}));
  const binary=project+'/cli.cjs';
  writeFileSync(binary,`#!/usr/bin/env node
const fs=require('node:fs');let input='';process.stdin.on('data',s=>input+=s);process.stdin.on('end',()=>{
 const p=process.argv[3];fs.appendFileSync(p+'/calls',JSON.stringify({argv:process.argv.slice(2),input:input?JSON.parse(input):null})+'\\n');
 const mode=fs.existsSync(p+'/mode')?fs.readFileSync(p+'/mode','utf8'):'';
  if(mode==='error'){console.error(JSON.stringify({error:{code:'stale_execution',message:'Read current state'}}));process.exit(2);}
  if(mode.startsWith('error:')){console.error(JSON.stringify({error:{code:mode.slice(6),message:'Select a valid unfinished run'}}));process.exit(2);}
 if(mode==='bad'){console.log('not JSON');return;}if(mode==='wait'){setInterval(()=>{},1000);return;}
 const output=fs.readFileSync(p+'/package','utf8');
 if(mode==='delay'&&['start','resume'].includes(process.argv[5])){const timer=setInterval(()=>{if(fs.existsSync(p+'/release')){clearInterval(timer);console.log(output);}},10);return;}
 console.log(output);
});`);chmodSync(binary,0o755);writeFileSync(project+'/package',JSON.stringify(pkg));
  const sessions=new Map([['ses_main',{id:'ses_main',directory:project,location:{directory:project},metadata:{keep:'user'}}]]);
  const hooks=new Map(),tools=new Map(),commands=new Map(),prompts=[];
  const f={project,binary,sessions,hooks,tools,prompts,calls:()=>existsSync(project+'/calls')?readFileSync(project+'/calls','utf8').trim().split('\n').map(JSON.parse):[]};
  const update=async(id,metadata)=>{if(metadata['unelos.opencode']?.runId)await f.beforeBind?.();sessions.get(id).metadata=metadata;};
  if(version==='v2') {
    await module.default.setup({options:{binary},tool:{transform:async fn=>fn({add:tool=>tools.set(tool.name,tool)}),hook:async(name,fn)=>hooks.set('tool.'+name,fn)},
      session:{get:async({sessionID})=>structuredClone(sessions.get(sessionID)),update:async({sessionID,metadata})=>{if(f.failUpdates)throw new Error('metadata unavailable');await update(sessionID,metadata);},prompt:async p=>prompts.push(p),hook:async(name,fn)=>hooks.set(name,fn)},
      command:{transform:async fn=>fn({add:c=>commands.set(c.name,c)})},model:{list:async()=>({data:[]})}});
    f.command=(text,sessionID='ses_main')=>commands.get('unelos').execute({sessionID,prompt:{text},delivery:'steer'});
    f.call=(name,args={},id='ses_main')=>tools.get(name).execute(args,{sessionID:id,signal:new AbortController().signal});
  } else {
    const native=await module.default.server({client:{session:{get:async({path})=>({data:structuredClone(sessions.get(path.id))}),update:async({path,body})=>{if(f.failUpdates)return {error:'metadata unavailable'};await update(path.id,body.metadata);return {data:sessions.get(path.id)};}}}}, {binary,v1ToolModule:process.env.UNELOS_V1_TOOL_MODULE});
    for(const [name,definition] of Object.entries(native.tool??{}))tools.set(name,definition);
    for(const [name,fn] of Object.entries(native))if(name!=='tool')hooks.set(name,fn);
    f.command=async(text,sessionID='ses_main')=>{const output={parts:[{type:'text',text:'Template'}]};await native['command.execute.before']({command:'unelos',sessionID,arguments:text},output);prompts.push(output);};
    f.call=(name,args={},id='ses_main')=>tools.get(name).execute(args,{sessionID:id,directory:project,abort:new AbortController().signal});
  }
  return f;
}

for(const version of ['v1','v2']) {
  test(`${version}: off/new cannot be overwritten between binding check and metadata write`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    for(const command of ['off','']){
      const f=await fixture(t,version);await f.command('');
      let entered,release;const paused=new Promise(resolve=>entered=resolve);const released=new Promise(resolve=>release=resolve);
      f.beforeBind=async()=>{entered();await released;};
      const start=f.call('unelos_start',{workflow:'page-change'});await paused;
      const newer=f.command(command);await delay(10);release();await Promise.all([start,newer]);
      const metadata=f.sessions.get('ses_main').metadata;
      assert.equal(metadata.keep,'user');
      if(command==='off')assert.equal(metadata['unelos.opencode'],undefined);
      else assert.equal(metadata['unelos.opencode'].runId,undefined);
      assert.equal(f.calls().filter(call=>call.argv.includes('start')).length,1);
    }
  });
  test(`${version}: two fresh sessions bind different runs and cross-run inputs never reach CLI`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);const packageFor=runId=>({...pkg,runId,expected:{...expected,runId}});
    f.sessions.set('ses_other',{...f.sessions.get('ses_main'),id:'ses_other',metadata:{keep:'other'}});
    await f.command('');assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','new','--json']);
    await assert.rejects(f.call('unelos_next'),{code:'run_unbound'});
    writeFileSync(f.project+'/package',JSON.stringify(packageFor('r-a')));await f.call('unelos_start',{workflow:'page-change'});
    await f.command('','ses_other');writeFileSync(f.project+'/package',JSON.stringify(packageFor('r-b')));await f.call('unelos_start',{workflow:'page-change'},'ses_other');
    await f.call('unelos_next');assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','next','--run','r-a','--json']);
    await f.call('unelos_next',{},'ses_other');assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','next','--run','r-b','--json']);
    const count=f.calls().length;
    for(const name of ['complete','approve','reject','continue'])await assert.rejects(f.call('unelos_'+name,{expected:{...expected,runId:'r-b'},resultText:'frame',progress:[]}),{code:'stale_execution'});
    await assert.rejects(f.call('unelos_rename',{name:'renamed',runId:'r-b'}),{code:'stale_execution'});
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),{code:'invalid_entry'});
    assert.equal(f.calls().length,count);assert.equal(f.sessions.get('ses_other').metadata['unelos.opencode'].runId,'r-b');
    assert.equal(f.sessions.get('ses_main').metadata.keep,'user');assert.equal(f.sessions.get('ses_other').metadata.keep,'other');
  });
  test(`${version}: explicit resume binds once and returns untouched context and worker prompt`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);await f.command('resume');assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','runs','--json']);
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),{code:'invalid_entry'});
    const resumed={...pkg,entryMode:'resume',resumeContext:{state:{runId:'r-1'},ledger:{content:'Existing brief'},reports:[{content:'Existing progress'}]}};
    writeFileSync(f.project+'/package',JSON.stringify(resumed));const count=f.calls().length;
    const output=await f.call('unelos_resume',{runId:'r-1'});const value=JSON.parse(version==='v2'?output.content:output);
    assert.equal(f.calls().length,count+1);assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','resume','--run','r-1','--json']);
    assert.deepEqual(value.resumeContext,resumed.resumeContext);assert.equal(value.entryMode,'resume');assert.equal(value.workerPrompt,pkg.workerPrompt);
    assert.equal(f.sessions.get('ses_main').metadata['unelos.opencode'].runId,'r-1');
  });
  test(`${version}: delayed start/resume cannot bind after off or a newer entry`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    for(const operation of ['start','resume'])for(const command of ['off','']){
      const f=await fixture(t,version);await f.command(operation==='resume'?'resume':'');writeFileSync(f.project+'/mode','delay');
      const count=f.calls().length;const pending=f.call('unelos_'+operation,operation==='start'?{workflow:'page-change'}:{runId:'r-1'});
      const rejected=assert.rejects(pending,error=>error.code===(operation==='start'?'mutation_uncertain':'entry_superseded'));
      for(let i=0;f.calls().length===count&&i<200;i++)await delay(10);
      assert.equal(f.calls().length,count+1);await f.command(command);
      if(command===''){writeFileSync(f.project+'/mode','');writeFileSync(f.project+'/package',JSON.stringify({...pkg,runId:'r-new'}));await f.call('unelos_start',{workflow:'page-change'});}
      const metadata=structuredClone(f.sessions.get('ses_main').metadata);
      writeFileSync(f.project+'/release','');await rejected;assert.deepEqual(f.sessions.get('ses_main').metadata,metadata);
    }
  });
  test(`${version}: failed metadata binding after start is uncertain and never retries`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);await f.command('');f.failUpdates=true;const count=f.calls().length;
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),error=>error.code==='mutation_uncertain'&&error.message.includes('r-1'));
    assert.equal(f.calls().length,count+1);assert.equal(f.sessions.get('ses_main').metadata['unelos.opencode'].runId,undefined);
  });
  test(`${version}: failed resume/entry never restores an old selection`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);await f.command('');await f.call('unelos_start',{workflow:'page-change'});
    for(const code of ['run_finished','unknown_run']){
      await f.command('resume');writeFileSync(f.project+'/mode','error:'+code);const count=f.calls().length;
      await assert.rejects(f.call('unelos_resume',{runId:'r-1'}),{code});assert.equal(f.calls().length,count+1);
      assert.equal(f.sessions.get('ses_main').metadata['unelos.opencode'].runId,undefined);
      await assert.rejects(f.call('unelos_status'),{code:'run_unbound'});writeFileSync(f.project+'/mode','');
    }
    writeFileSync(f.project+'/mode','error:cli_failed');await assert.rejects(f.command(''),{code:'cli_failed'});
    assert.equal(f.sessions.get('ses_main').metadata['unelos.opencode'].runId,undefined);
  });
  test(`${version}: native tools use one CLI call, exact core prompt and no worker/request hooks`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);assert.deepEqual([...f.tools.keys()].sort(),names.toSorted());
    await f.command('');assert.equal(f.calls().length,1);
    await f.call('unelos_start',{workflow:'page-change'});
    const before=f.calls().length;const output=await f.call('unelos_complete',{expected,resultText:'Worker frame'});
    const value=JSON.parse(version==='v2'?output.content:output);assert.equal(value.workerPrompt,pkg.workerPrompt);
    assert.equal(value.nativeAgent,'unelos-engineer');assert.equal(f.calls().length,before+1);
    assert.deepEqual(f.calls().at(-1).argv.slice(2),['execution','complete','--run','r-1','--json']);
    assert.deepEqual(f.calls().at(-1).input,{expected,resultText:'Worker frame',outcome:null,reason:null});
    assert.ok(![...f.hooks.keys()].some(name=>/context|chat\.params|tool\.execute|messages\.transform/.test(name)));
    assert.equal(f.sessions.get('ses_main').metadata.keep,'user');
    const cases=[['unelos_next',{},['execution','next','--run','r-1','--json']],['unelos_approve',{expected},['execution','approve','--run','r-1','--json']],['unelos_reject',{expected,reason:'Revise'},['execution','reject','--run','r-1','--json']],['unelos_continue',{expected,answer:'Yes',progress:[]},['execution','continue','--run','r-1','--json']],['unelos_status',{},['status','--run','r-1','--json']],['unelos_inspect',{workflow:'page-change',node:'plan'},['inspect','page-change','--node','plan','--json']],['unelos_rename',{name:'renamed',runId:'r-1'},['rename','renamed','--run','r-1','--json']]];
    for(const [name,args,argv] of cases){const count=f.calls().length;await f.call(name,args);assert.equal(f.calls().length,count+1);assert.deepEqual(f.calls().at(-1).argv.slice(2),argv);}
  });
  test(`${version}: ordinary/child/off sessions cannot progress and failures never retry`,{skip:version==='v1'&&!process.env.UNELOS_V1_TOOL_MODULE},async t=>{
    const f=await fixture(t,version);assert.equal(f.calls().length,0);
    await assert.rejects(f.call('unelos_complete',{expected,resultText:'Frame'}),{code:'controller_inactive'});assert.equal(f.calls().length,0);
    await f.command('');await f.call('unelos_start',{workflow:'page-change'});f.sessions.set('ses_child',{...f.sessions.get('ses_main'),id:'ses_child',parentID:'ses_main'});
    await assert.rejects(f.call('unelos_complete',{expected,resultText:'Frame'},'ses_child'),{code:'child_controller'});
    writeFileSync(f.project+'/mode','error');const count=f.calls().length;
    await assert.rejects(f.call('unelos_approve',{expected}),{code:'stale_execution'});assert.equal(f.calls().length,count+1);
    await f.command('off');await assert.rejects(f.call('unelos_next'),{code:'controller_inactive'});assert.equal(f.calls().length,count+1);
  });
}

test('transport preserves safe errors, invalid JSON and cancellation without retries',async t=>{
  const f=await fixture(t,'v2');
  writeFileSync(f.project+'/mode','bad');await assert.rejects(module.callCli(f.binary,f.project,['execution','next','--json']),{code:'invalid_cli_json'});
  writeFileSync(f.project+'/mode','wait');const controller=new AbortController();
  const pending=module.callCli(f.binary,f.project,['execution','next','--json'],undefined,controller.signal);controller.abort();
  await assert.rejects(pending,{code:'cli_cancelled'});
});

test('V1 missing explicit native helper fails clearly instead of inventing schemas',async()=>{
  await assert.rejects(module.default.server({client:{}},{v1ToolModule:'/tmp/opencode/nonexistent-native-helper.js'}),{code:'native_tool_sdk_unavailable'});
});

test('transport times out a hung CLI without retrying',async t=>{
  const f=await fixture(t,'v2');writeFileSync(f.project+'/mode','wait');
  await assert.rejects(module.callCli(f.binary,f.project,['execution','next','--json']),{code:'cli_failed'});
  assert.equal(f.calls().length,1);
});

test('v2: parallel package maps every worker and forwards workerKey continuations',async t=>{
  const f=await fixture(t,'v2');
  const parallel={kind:'parallel',runId:'r-1',nodeId:'parallel-research',taskId:'task-4'};
  assert.ok(module.toolSchemas.complete.properties.expected.properties.kind.enum.includes('parallel'));
  assert.deepEqual(module.toolSchemas.continue.properties.workerKey,{type:'string'});
  writeFileSync(f.project+'/package',JSON.stringify({...pkg,expected:parallel,packet:{kind:'parallel'},workerPrompt:null,
    workerPrompts:[{key:'worker-1',agent:'backend-researcher',prompt:'B'},{key:'worker-2',agent:'frontend-researcher',prompt:'F'}]}));
  await f.command('');await f.call('unelos_start',{workflow:'parallel-research'});
  const value=JSON.parse((await f.call('unelos_continue',{expected:parallel,workerKey:'worker-2',answer:'Yes',progress:[]})).content);
  assert.deepEqual(value.workerPrompts.map(w=>[w.key,w.prompt,w.nativeAgent]),[['worker-1','B','unelos-backend-researcher'],['worker-2','F','unelos-frontend-researcher']]);
  assert.equal(value.nativeAgent,undefined);
  assert.match(value.harnessInstructions,/background=true/);
  assert.deepEqual(f.calls().at(-1).input,{expected:parallel,workerKey:'worker-2',answer:'Yes',progress:[]});
});
