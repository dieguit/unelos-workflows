import { spawn } from 'node:child_process';
import { realpath } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';

const OWNER='unelos.opencode';
const fail=(code,message)=>Object.assign(new Error(message),{code});
const object=(properties={},required=[])=>({type:'object',properties,required,additionalProperties:false});
const string={type:'string'};
const identity=object({kind:{type:'string',enum:['task','human-gate']},runId:string,nodeId:string,taskId:string,activationId:string},['kind','runId','nodeId']);
const descriptions={next:'Read the selected Unelos execution package',start:'Start a fresh Unelos workflow and bind this session',resume:'Bind the explicitly selected unfinished run and immediately continue using core resume context',complete:'Complete the expected Unelos Task after inspection',approve:'Record explicit approval of the expected human gate',reject:'Reject the expected human gate with actionable feedback',continue:'Prepare a same-Task worker continuation',status:'Read selected Unelos status',inspect:'Inspect a Unelos workflow or Node',rename:'Name the selected Unelos run'};

export const toolSchemas={
  next:object(),start:object({workflow:string,name:string},['workflow']),resume:object({runId:string},['runId']),
  complete:object({expected:identity,resultText:string,outcome:string},['expected','resultText']),
  approve:object({expected:identity},['expected']),reject:object({expected:identity,reason:string},['expected','reason']),
  continue:object({expected:identity,answer:string,progress:{type:'array',items:string}},['expected','progress']),
  status:object(),inspect:object({workflow:string,node:string},['workflow']),rename:object({name:string,runId:string},['name','runId']),
};

export function callCli(binary,project,argv,input,signal) {
  const mutation=argv[0]==='rename'||argv[0]==='execution'&&['start','complete','approve','reject'].includes(argv[1]);
  const target=argv.includes('--run')?argv[argv.indexOf('--run')+1]:null;
  const guidance=target?`Reconcile with execution next --run ${target}; never blindly retry`:'List unfinished runs and explicitly reconcile; never blindly retry';
  return new Promise((resolve,reject)=>{
    const child=spawn(binary,['--project',project,...argv],{signal,timeout:10000,killSignal:'SIGKILL',stdio:['pipe','pipe','pipe']});
    let stdout='',stderr='',size=0;
    const uncertain=()=>fail(mutation?'mutation_uncertain':'cli_failed',mutation?`Mutation response unavailable; ${guidance}`:'Unelos CLI failed');
    for(const [stream,append] of [[child.stdout,s=>stdout+=s],[child.stderr,s=>stderr+=s]]) {
      stream.setEncoding('utf8');stream.on('data',s=>{
        size+=Buffer.byteLength(s);if(size>16*1024*1024){child.kill('SIGKILL');reject(uncertain());}else append(s);
      });
    }
    child.once('error',error=>reject(error.name==='AbortError'
      ? fail(mutation?'mutation_uncertain':'cli_cancelled',mutation?`Mutation interrupted; ${guidance}`:'CLI operation cancelled') : uncertain()));
    child.stdin.on('error',()=>{}); // Process exit/close determines the operation's result.
    child.once('close',code=>{
      if(code!==0){
        for(const line of stderr.trim().split('\n').reverse()) {
          try {const error=JSON.parse(line)?.error;
            if(typeof error?.code==='string'&&typeof error.message==='string'){reject(fail(error.code,error.message));return;}
          }catch{}
        }
        reject(uncertain());return;
      }
      try {const value=JSON.parse(stdout);
        if(!value||typeof value!=='object'||Array.isArray(value))throw new Error();resolve(value);
      }catch{reject(fail(mutation?'mutation_uncertain':'invalid_cli_json',mutation?`Mutation response malformed; ${guidance}`:'Unelos returned invalid JSON'));}
    });
    child.stdin.end(input===undefined?undefined:JSON.stringify(input));
  });
}

function binaryOption(options) {
  const binary=options?.binary??'unelos';
  if(typeof binary!=='string'||!binary.trim()||binary.includes('\0'))throw fail('invalid_binary','Specify one CLI executable');
  return binary;
}

function mapping(version) {
  return 'Native Unelos tools: '+Object.keys(toolSchemas).map(name=>'unelos_'+name).join(', ')+'.\n'
    +(version==='v1'?'Native worker: task(subagent_type=nativeAgent, prompt=workerPrompt, task_id for continuation).'
      :'Native worker: subagent(agent=nativeAgent, prompt=workerPrompt, sessionID for continuation).');
}

function nativePackage(value,version) {
  return {...value,...(value.workerPrompt&&value.packet?.agent?{nativeAgent:'unelos-'+value.packet.agent}:{}),harnessInstructions:mapping(version)};
}

function bridge(binary,version,get,update,directory) {
  const changes=new Map(); // Only serialize metadata check/write; CLI work never holds this queue.
  function change(id,fn) {
    const next=(changes.get(id)??Promise.resolve()).catch(()=>{}).then(fn);
    changes.set(id,next);
    return next.finally(()=>{if(changes.get(id)===next)changes.delete(id);});
  }
  async function session(id) {
    const value=await get(id);if(value?.id!==id)throw fail('session_lookup_failed','Native session unavailable');
    if(value.parentID!=null)throw fail('child_controller','Workers cannot operate the Unelos controller');
    return value;
  }
  async function project(id) {
    const value=await session(id);const active=value.metadata?.[OWNER];
    if(!active?.enabled)throw fail('controller_inactive','Activate /unelos in the main session first');
    const path=await realpath(directory(value));
    if(active.project!==path)throw fail('project_changed','Project changed; reactivate /unelos');
    return {path,active,value};
  }
  async function current(id,active) {
    const state=await project(id);
    if(state.active.entryId!==active.entryId)throw fail('entry_superseded','Entry superseded; explicitly select a run again');
    return state.value;
  }
  return {
    async command(id,text) {
      const words=text.trim().split(/\s+/).filter(Boolean);
      if(words.length&&!(words.length===1&&['off','resume'].includes(words[0])))throw fail('invalid_command','Usage: /unelos [resume | off]');
      const entry=await change(id,async()=>{
        const value=await session(id);const path=await realpath(directory(value));const metadata={...value.metadata};
        if(words[0]==='off'){delete metadata[OWNER];await update(id,metadata);return null;}
        const active={project:path,enabled:true,mode:words[0]==='resume'?'resume':'new',entryId:randomUUID()};
        await update(id,{...metadata,[OWNER]:active});return {path,active};
      });
      if(!entry)return 'Unelos disabled for this session.';
      const {path,active}=entry;
      const result=nativePackage(await callCli(binary,path,['execution',active.mode==='new'?'new':'runs','--json']),version);
      await current(id,active);
      return JSON.stringify({...result,requestedAction:text});
    },
    async execute(name,input,context) {
      const {path,active}=await project(context.sessionID);let argv,control;
      const entry=['start','resume'].includes(name);
      if(entry){
        if(active.runId||active.mode!==(name==='start'?'new':'resume'))throw fail('invalid_entry','Use /unelos for a fresh start or /unelos resume to select an existing run');
      }else if(name!=='inspect'){
        if(!active.runId)throw fail('run_unbound','Start or explicitly resume a run first');
        if(input.expected&&input.expected.runId!==active.runId||name==='rename'&&input.runId!==active.runId)throw fail('stale_execution','Input identity does not match this session\'s selected run');
      }
      switch(name){
        case 'next':argv=['execution','next','--run',active.runId];break;
        case 'start':argv=['execution','start',input.workflow,...(input.name?['--name',input.name]:[])];break;
        case 'resume':argv=['execution','resume','--run',input.runId];break;
        case 'complete':case 'approve':case 'reject':
          argv=['execution',name,'--run',active.runId];control={expected:input.expected,resultText:input.resultText??null,outcome:input.outcome??null,reason:input.reason??null};break;
        case 'continue':argv=['execution','continue','--run',active.runId];control={expected:input.expected,answer:input.answer??null,progress:input.progress};break;
        case 'status':argv=['status','--run',active.runId];break;
        case 'inspect':argv=['inspect',input.workflow,...(input.node?['--node',input.node]:[])];break;
        case 'rename':argv=['rename',input.name,'--run',active.runId];break;
        default:throw fail('invalid_tool','Unknown Unelos tool');
      }
      const result=await callCli(binary,path,[...argv,'--json'],control,context.signal??context.abort);
      if(entry){
        try{
          if(typeof result.runId!=='string'||!result.runId.trim()||name==='resume'&&result.runId!==input.runId)throw fail('invalid_cli_json','CLI returned no valid selected run ID');
          await change(context.sessionID,async()=>{
            const value=await current(context.sessionID,active);
            if(value.metadata[OWNER].runId)throw fail('entry_superseded','Entry already bound; select a run again');
            await update(context.sessionID,{...value.metadata,[OWNER]:{...active,runId:result.runId}});
          });
        }catch(error){
          if(name==='start')throw fail('mutation_uncertain',`Run ${result.runId??'(unknown)'} may have been created but binding failed; explicitly reconcile with /unelos resume; never retry start`);
          if(['controller_inactive','project_changed','entry_superseded'].includes(error.code))throw fail('entry_superseded','Resume entry superseded; select a run again');
          throw error;
        }
      }
      return nativePackage(result,version);
    },
  };
}

export default {
  id:OWNER,
  async setup(ctx) {
    const native=bridge(binaryOption(ctx.options),'v2',id=>ctx.session.get({sessionID:id}),
      (id,metadata)=>ctx.session.update({sessionID:id,metadata}),s=>s.location.directory);
    await ctx.tool.transform(editor=>{
      for(const [name,input] of Object.entries(toolSchemas))editor.add({name:'unelos_'+name,description:descriptions[name],input,
        execute:async(args,context)=>({content:JSON.stringify(await native.execute(name,args,context))})});
    });
    await ctx.command.transform(editor=>editor.add({name:'unelos',description:'Activate the Unelos controller',
      execute:async({sessionID,prompt,delivery})=>ctx.session.prompt({...prompt,sessionID,delivery,text:await native.command(sessionID,prompt.text)})}));
  },
  async server({client},options) {
    const get=async id=>{const result=await client.session.get({path:{id}});if(result.error)throw fail('session_lookup_failed','Native session unavailable');return result.data;};
    const native=bridge(binaryOption(options),'v1',get,async(id,metadata)=>{
      const result=await client.session.update({path:{id},body:{metadata}});if(result.error)throw fail('activation_failed','Native activation failed');
    },s=>s.directory);
    const {registerV1}=await import('./v1.js');
    return {tool:await registerV1(options,descriptions,native.execute),
      'command.execute.before':async(input,output)=>{
        if(input.command!=='unelos')return;
        const part=output.parts.find(p=>p.type==='text');if(!part)throw fail('delivery_text_missing','Command needs text');
        part.text=await native.command(input.sessionID,input.arguments);
      }};
  },
};
