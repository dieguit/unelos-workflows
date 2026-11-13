import {test} from 'vitest';
import type {TestContext} from 'vitest';
import type {Plugin} from '@opencode/plugin';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,chmodSync,existsSync,rmSync} from 'node:fs';
import * as module from '../index.ts';
import {setTimeout as delay} from 'node:timers/promises';

type ToolEditor=Parameters<Parameters<Plugin.Context['tool']['transform']>[0]>[0];
type CommandEditor=Parameters<Parameters<Plugin.Context['command']['transform']>[0]>[0];
type FixtureSession={id:string;directory:string;location:{directory:string};parentID?:string;metadata:{keep:string;'unelos.opencode'?:{runId?:string}}};
type CliCall={argv:string[];input:unknown};
type Prompt=Parameters<Plugin.Context['session']['prompt']>[0];
const names=['unelos_next','unelos_start','unelos_resume','unelos_complete','unelos_approve','unelos_reject','unelos_status','unelos_inspect','unelos_rename'];
const expected={kind:'task',runId:'r-1',nodeId:'implement',taskId:'task-3'};
const pkg={runId:'r-1',status:'running',workflows:[],expected,packet:{agent:'engineer'},workerPrompt:'Exact core prompt\nSELECTED BODY',controllerInstructions:'Shared controller policy'};

async function fixture(t:TestContext) {
  const project=mkdtempSync(join(tmpdir(),'unelos-thin-tools-'));t.onTestFinished(()=>rmSync(project,{recursive:true}));
  const binary=project+'/cli.cjs';
  writeFileSync(binary,`#!/usr/bin/env node
const fs=require('node:fs');let input='';process.stdin.on('data',s=>input+=s);process.stdin.on('end',()=>{
 const p=process.argv[3];fs.appendFileSync(p+'/calls',JSON.stringify({argv:process.argv.slice(2),input:input?JSON.parse(input):null})+'\\n');
 const mode=fs.existsSync(p+'/mode')?fs.readFileSync(p+'/mode','utf8'):'';
  if(mode==='error'){console.error(JSON.stringify({error:{code:'stale_execution',message:'Read current state'}}));process.exit(2);}
  if(mode.startsWith('error:')){console.error(JSON.stringify({error:{code:mode.slice(6),message:'Select a valid unfinished run'}}));process.exit(2);}
 if(mode==='bad'){console.log('not JSON');return;}if(mode==='wait'){setInterval(()=>{},1000);return;}
  const output=fs.readFileSync(p+'/package','utf8');
  if(process.argv[4]==='inspect'){console.log(fs.existsSync(p+'/inspection')?fs.readFileSync(p+'/inspection','utf8'):JSON.stringify({name:'page-change',nodes:[{id:'implement',kind:'task'}],edges:[]}));return;}
 if(mode==='delay'&&['start','resume'].includes(process.argv[5])){const timer=setInterval(()=>{if(fs.existsSync(p+'/release')){clearInterval(timer);console.log(output);}},10);return;}
 console.log(output);
});`);chmodSync(binary,0o755);writeFileSync(project+'/package',JSON.stringify(pkg));
  const sessions=new Map<string,FixtureSession>([['ses_main',{id:'ses_main',directory:project,location:{directory:project},metadata:{keep:'user'}}]]);
  const hooks=new Map<string,unknown>(),tools=new Map<string,Parameters<ToolEditor['add']>[0]>(),commands=new Map<string,Parameters<CommandEditor['add']>[0]>(),prompts:Prompt[]=[];
  const f:{project:string;binary:string;sessions:typeof sessions;hooks:typeof hooks;tools:typeof tools;prompts:Prompt[];calls:()=>CliCall[];beforeBind?:()=>Promise<void>;failUpdates?:boolean;command:(text:string,sessionID?:string)=>Promise<void>;call:(name:string,args?:unknown,id?:string)=>Promise<{content:string}>}={project,binary,sessions,hooks,tools,prompts,calls:()=>existsSync(project+'/calls')?readFileSync(project+'/calls','utf8').trim().split('\n').map(line=>JSON.parse(line)):[],command:async()=>{},call:async()=>({content:''})};
  const update=async(id:string,metadata:Parameters<Plugin.Context['session']['update']>[0]['metadata'])=>{const next=metadata as FixtureSession['metadata'];if(next['unelos.opencode']?.runId)await f.beforeBind?.();sessions.get(id)!.metadata=next;};
  const ctx:Pick<Plugin.Context,'options'|'tool'|'session'|'command'|'model'>={
    options:{binary},
    tool:{transform:async(fn:Parameters<Plugin.Context['tool']['transform']>[0])=>{fn({add:tool=>{tools.set(tool.name,tool);}} as Parameters<typeof fn>[0]);},hook:async(name:string,fn:unknown)=>{hooks.set('tool.'+name,fn);}},
    session:{get:async({sessionID}:Parameters<Plugin.Context['session']['get']>[0])=>structuredClone(sessions.get(sessionID)),update:async({sessionID,metadata}:Parameters<Plugin.Context['session']['update']>[0])=>{if(f.failUpdates)throw new Error('metadata unavailable');await update(sessionID,metadata);},prompt:async(p:Parameters<Plugin.Context['session']['prompt']>[0])=>{prompts.push(p);},hook:async(name:string,fn:unknown)=>{hooks.set(name,fn);}},
    command:{transform:async(fn:Parameters<Plugin.Context['command']['transform']>[0])=>{fn({add:c=>{commands.set(c.name,c);}});}},
    model:{list:async()=>({data:[]})},
  } as unknown as Pick<Plugin.Context,'options'|'tool'|'session'|'command'|'model'>;
  await module.default.setup(ctx as Plugin.Context);
  f.command=(text,sessionID='ses_main')=>commands.get('unelos')!.execute({sessionID,prompt:{text},delivery:'steer'} as Parameters<Parameters<CommandEditor['add']>[0]['execute']>[0]);
  f.call=async(name,args={},id='ses_main')=>await tools.get(name)!.execute(args,{sessionID:id,signal:new AbortController().signal} as Parameters<Parameters<ToolEditor['add']>[0]['execute']>[1]) as {content:string};
  return f;
}

for(const version of ['v2']) {
  test(`${version}: free text is forwarded and only single-word keywords are special`,async t=>{
    const f=await fixture(t);
    for(const text of ['build a dashboard','resume this idea','off the beaten path']){
      await f.command(text);
      const value=JSON.parse(f.prompts.at(-1)!.text);
      assert.equal(value.requestedAction,text);
      assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','new','--json']);
    }
  });
  test(`${version}: off/new cannot be overwritten between binding check and metadata write`,async t=>{
    for(const command of ['off','']){
      const f=await fixture(t);await f.command('');
      let entered!:()=>void,release!:()=>void;const paused=new Promise<void>(resolve=>entered=resolve);const released=new Promise<void>(resolve=>release=resolve);
      f.beforeBind=async()=>{entered();await released;};
      const start=f.call('unelos_start',{workflow:'page-change'});await paused;
      const newer=f.command(command);await delay(10);release();await Promise.all([start,newer]);
      const metadata=f.sessions.get('ses_main')!.metadata;
      assert.equal(metadata.keep,'user');
      if(command==='off')assert.equal(metadata['unelos.opencode'],undefined);
      else assert.equal(metadata['unelos.opencode']!.runId,undefined);
      assert.equal(f.calls().filter(call=>call.argv.includes('start')).length,1);
    }
  });
  test(`${version}: two fresh sessions bind different runs and cross-run inputs never reach CLI`,async t=>{
    const f=await fixture(t);const packageFor=(runId:string)=>({...pkg,runId,expected:{...expected,runId}});
    f.sessions.set('ses_other',{...f.sessions.get('ses_main')!,id:'ses_other',metadata:{keep:'other'}});
    await f.command('');assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','new','--json']);
    await assert.rejects(f.call('unelos_next'),{code:'run_unbound'});
    writeFileSync(f.project+'/package',JSON.stringify(packageFor('r-a')));await f.call('unelos_start',{workflow:'page-change'});
    await f.command('','ses_other');writeFileSync(f.project+'/package',JSON.stringify(packageFor('r-b')));await f.call('unelos_start',{workflow:'page-change'},'ses_other');
    await f.call('unelos_next');assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','next','--run','r-a','--json']);
    await f.call('unelos_next',{},'ses_other');assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','next','--run','r-b','--json']);
    const count=f.calls().length;
    for(const name of ['complete','approve','reject'])await assert.rejects(f.call('unelos_'+name,{expected:{...expected,runId:'r-b'},resultText:'frame',progress:[]}),{code:'stale_execution'});
    await assert.rejects(f.call('unelos_rename',{name:'renamed',runId:'r-b'}),{code:'stale_execution'});
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),{code:'invalid_entry'});
    assert.equal(f.calls().length,count);assert.equal(f.sessions.get('ses_other')!.metadata['unelos.opencode']!.runId,'r-b');
    assert.equal(f.sessions.get('ses_main')!.metadata.keep,'user');assert.equal(f.sessions.get('ses_other')!.metadata.keep,'other');
  });
  test(`${version}: explicit resume binds once and returns untouched context and worker prompt`,async t=>{
    const f=await fixture(t);await f.command('resume');assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','runs','--json']);
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),{code:'invalid_entry'});
    const resumed={...pkg,entryMode:'resume',resumeContext:{state:{runId:'r-1'},ledger:{content:'Existing brief'},reports:[{content:'Existing progress'}]}};
    writeFileSync(f.project+'/package',JSON.stringify(resumed));const count=f.calls().length;
    const output=await f.call('unelos_resume',{runId:'r-1'});const value=JSON.parse(output.content);
    assert.equal(f.calls().length,count+1);assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','resume','--run','r-1','--json']);
    assert.deepEqual(value.resumeContext,resumed.resumeContext);assert.equal(value.entryMode,'resume');assert.equal(value.workerPrompt,pkg.workerPrompt);
    assert.equal(f.sessions.get('ses_main')!.metadata['unelos.opencode']!.runId,'r-1');
  });
  test(`${version}: delayed start/resume cannot bind after off or a newer entry`,async t=>{
    for(const operation of ['start','resume'])for(const command of ['off','']){
      const f=await fixture(t);await f.command(operation==='resume'?'resume':'');writeFileSync(f.project+'/mode','delay');
      const count=f.calls().length;const pending=f.call('unelos_'+operation,operation==='start'?{workflow:'page-change'}:{runId:'r-1'});
      const rejected=assert.rejects(pending,(error:unknown)=>error instanceof Error && 'code' in error && error.code===(operation==='start'?'mutation_uncertain':'entry_superseded'));
      const extra=1;
      for(let i=0;f.calls().length<count+extra&&i<200;i++)await delay(10);
      assert.equal(f.calls().length,count+extra);await f.command(command);
      if(command===''){writeFileSync(f.project+'/mode','');writeFileSync(f.project+'/package',JSON.stringify({...pkg,runId:'r-new'}));await f.call('unelos_start',{workflow:'page-change'});}
      const metadata=structuredClone(f.sessions.get('ses_main')!.metadata);
      writeFileSync(f.project+'/release','');await rejected;assert.deepEqual(f.sessions.get('ses_main')!.metadata,metadata);
    }
  });
  test(`${version}: failed metadata binding after start is uncertain and never retries`,async t=>{
    const f=await fixture(t);await f.command('');f.failUpdates=true;const count=f.calls().length;
    await assert.rejects(f.call('unelos_start',{workflow:'page-change'}),(error:unknown)=>error instanceof Error && 'code' in error && error.code==='mutation_uncertain'&&error.message.includes('r-1'));
    assert.equal(f.calls().length,count+1);assert.equal(f.sessions.get('ses_main')!.metadata['unelos.opencode']!.runId,undefined);
  });
  test(`${version}: failed resume/entry never restores an old selection`,async t=>{
    const f=await fixture(t);await f.command('');await f.call('unelos_start',{workflow:'page-change'});
    for(const code of ['run_finished','unknown_run']){
      await f.command('resume');writeFileSync(f.project+'/mode','error:'+code);const count=f.calls().length;
      await assert.rejects(f.call('unelos_resume',{runId:'r-1'}),{code});assert.equal(f.calls().length,count+1);
      assert.equal(f.sessions.get('ses_main')!.metadata['unelos.opencode']!.runId,undefined);
      await assert.rejects(f.call('unelos_status'),{code:'run_unbound'});writeFileSync(f.project+'/mode','');
    }
    writeFileSync(f.project+'/mode','error:cli_failed');await assert.rejects(f.command(''),{code:'cli_failed'});
    assert.equal(f.sessions.get('ses_main')!.metadata['unelos.opencode']!.runId,undefined);
  });
  test(`${version}: native tools use one CLI call, exact core prompt and no worker/request hooks`,async t=>{
    const f=await fixture(t);assert.deepEqual([...f.tools.keys()].sort(),names.toSorted());
    await f.command('');assert.equal(f.calls().length,1);
    await f.call('unelos_start',{workflow:'page-change'});
    const before=f.calls().length;const output=await f.call('unelos_complete',{expected,resultText:'Worker frame'});
    const value=JSON.parse(output.content);assert.equal(value.workerPrompt,pkg.workerPrompt);
    assert.equal(value.nativeAgent,'unelos-engineer');assert.equal(f.calls().length,before+1);
    assert.deepEqual(f.calls().at(-1)!.argv.slice(2),['execution','complete','--run','r-1','--json']);
    assert.deepEqual(f.calls().at(-1)!.input,{expected,resultText:'Worker frame',summary:null,outcome:null,reason:null});
    await f.call('unelos_complete',{expected,summary:'Main Task done',outcome:'fix'});
    assert.deepEqual(f.calls().at(-1)!.input,{expected,resultText:null,summary:'Main Task done',outcome:'fix',reason:null});
    assert.ok(![...f.hooks.keys()].some(name=>/context|chat\.params|tool\.execute|messages\.transform/.test(name)));
    assert.equal(f.sessions.get('ses_main')!.metadata.keep,'user');
    const cases:[string,unknown,string[]][]=[['unelos_next',{},['execution','next','--run','r-1','--json']],['unelos_approve',{expected},['execution','approve','--run','r-1','--json']],['unelos_reject',{expected,reason:'Revise'},['execution','reject','--run','r-1','--json']],['unelos_status',{},['status','--run','r-1','--json']],['unelos_inspect',{workflow:'page-change',node:'plan'},['inspect','page-change','--node','plan','--json']],['unelos_rename',{name:'renamed',runId:'r-1'},['rename','renamed','--run','r-1','--json']]];
    for(const [name,args,argv] of cases){const count=f.calls().length;await f.call(name,args);assert.equal(f.calls().length,count+1);assert.deepEqual(f.calls().at(-1)!.argv.slice(2),argv);}
  });
  test(`${version}: ordinary/child/off sessions cannot progress and failures never retry`,async t=>{
    const f=await fixture(t);assert.equal(f.calls().length,0);
    await assert.rejects(f.call('unelos_complete',{expected,resultText:'Frame'}),{code:'controller_inactive'});assert.equal(f.calls().length,0);
    await f.command('');await f.call('unelos_start',{workflow:'page-change'});f.sessions.set('ses_child',{...f.sessions.get('ses_main')!,id:'ses_child',parentID:'ses_main'});
    await assert.rejects(f.call('unelos_complete',{expected,resultText:'Frame'},'ses_child'),{code:'child_controller'});
    writeFileSync(f.project+'/mode','error');const count=f.calls().length;
    await assert.rejects(f.call('unelos_approve',{expected}),{code:'stale_execution'});assert.equal(f.calls().length,count+1);
    await f.command('off');await assert.rejects(f.call('unelos_next'),{code:'controller_inactive'});assert.equal(f.calls().length,count+1);
  });
}

test('transport preserves safe errors, invalid JSON and cancellation without retries',async t=>{
  const f=await fixture(t);
  writeFileSync(f.project+'/mode','bad');await assert.rejects(module.callCli(f.binary,f.project,['execution','next','--json']),{code:'invalid_cli_json'});
  writeFileSync(f.project+'/mode','wait');const controller=new AbortController();
  const pending=module.callCli(f.binary,f.project,['execution','next','--json'],undefined,controller.signal);controller.abort();
  await assert.rejects(pending,{code:'cli_cancelled'});
});

test('transport times out a hung CLI without retrying',{timeout:15000},async t=>{
  const f=await fixture(t);writeFileSync(f.project+'/mode','wait');
  await assert.rejects(module.callCli(f.binary,f.project,['execution','next','--json']),{code:'cli_failed'});
  assert.equal(f.calls().length,1);
});

test('v2: parallel package maps every worker without continuation tool',async t=>{
  const f=await fixture(t);
  const parallel={kind:'parallel',runId:'r-1',nodeId:'parallel-research',taskId:'task-4'};
  assert.ok(module.toolSchemas.complete.properties.expected.properties.kind.enum.includes('parallel'));
  assert.equal('continue' in module.toolSchemas,false);
  assert.deepEqual(module.toolSchemas.complete.required,['expected']);
  assert.match(module.toolSchemas.complete.properties.resultText.description,/verbatim/);
  assert.match(module.toolSchemas.complete.properties.outcome.description,/main Tasks only/);
  writeFileSync(f.project+'/package',JSON.stringify({...pkg,expected:parallel,packet:{kind:'parallel'},workerPrompt:null,
    workerPrompts:[{key:'worker-1',agent:'backend-researcher',prompt:'B'},{key:'worker-2',agent:'frontend-researcher',prompt:'F'}]}));
  await f.command('');await f.call('unelos_start',{workflow:'parallel-research'});
  const value=JSON.parse((await f.call('unelos_complete',{expected:parallel,resultText:'B final\nF final'})).content);
  assert.deepEqual(value.workerPrompts.map((w:{key:string;prompt:string;nativeAgent:string})=>[w.key,w.prompt,w.nativeAgent]),[['worker-1','B','unelos-backend-researcher'],['worker-2','F','unelos-frontend-researcher']]);
  assert.equal(value.nativeAgent,undefined);
  assert.match(value.harnessInstructions,/background=true/);
  assert.deepEqual(f.calls().at(-1)!.input,{expected:parallel,resultText:'B final\nF final',summary:null,outcome:null,reason:null});
});
