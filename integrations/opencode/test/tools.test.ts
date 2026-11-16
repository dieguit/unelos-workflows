import {test} from 'vitest';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,rmSync,realpathSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import type {Plugin} from '@opencode/plugin';
import plugin from '../index.ts';

test('MCP registration, trusted context replacement and session activation',async t=>{
  const project=mkdtempSync(join(tmpdir(),'unelos-mcp-'));
  t.onTestFinished(()=>rmSync(project,{recursive:true,force:true}));
  const binary=join(project,'cli.cjs');
  writeFileSync(binary,'#!/usr/bin/env node\nconsole.log(JSON.stringify({argv:process.argv.slice(2),controllerInstructions:"core",harnessInstructions:"adapter"}));',{mode:0o755});
  type Call=(input:unknown)=>Promise<void>;
  let hook!:Call,command!:{execute:Call},registration:unknown,prompt!:{text:string;delivery?:string};
  const session={id:'ses_main',location:{directory:project},parentID:null as string|null};
  await plugin.setup({
    options:{binary},
    mcp:{transform:async(fn:(editor:unknown)=>void)=>fn({set:(name:string,value:unknown)=>{registration={name,value};}})},
    tool:{hook:async(name:string,fn:Call)=>{assert.equal(name,'execute.before');hook=fn;}},
    session:{get:async()=>session,prompt:async(value:typeof prompt)=>{prompt=value;}},
    command:{transform:async(fn:(editor:unknown)=>void)=>fn({add:(value:typeof command)=>{command=value;}})},
  } as unknown as Plugin.Context);
  assert.deepEqual(registration,{name:'unelos',value:{type:'local',command:[binary,'mcp']}});
  for(const parentID of [null,'ses_parent']){
    session.parentID=parentID;
    const input={workflow:'demo',unelos_context:{harness:'spoof',extra:true}};
    await hook({tool:'unelos_start',sessionID:'ses_main',input});
    assert.deepEqual(input.unelos_context,{harness:'opencode',sessionId:'ses_main',project:realpathSync(project),role:parentID!==null?'worker':'controller'});
    assert.equal(input.workflow,'demo');
  }
  const untouched={unelos_context:{custom:true}};
  await hook({tool:'other',sessionID:'ses_main',input:untouched});
  assert.deepEqual(untouched,{unelos_context:{custom:true}});
  await assert.rejects(()=>command.execute({sessionID:'ses_main',prompt:{text:''}}),{code:'child_controller'});
  session.parentID=null;
  for(const text of ['','resume','off','resume this idea']){
    await command.execute({sessionID:'ses_main',prompt:{text},delivery:'steer'});
    const result=JSON.parse(prompt.text);
    assert.deepEqual(result.argv,['--project',realpathSync(project),'session','activate','--harness','opencode','--session','ses_main','--request',text,'--json']);
    assert.equal(result.requestedAction,text);
    assert.equal(result.controllerInstructions,'core');
    assert.equal(result.harnessInstructions,'adapter');
    assert.equal(prompt.delivery,'steer');
  }
});
