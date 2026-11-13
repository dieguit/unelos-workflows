import { spawn } from 'node:child_process';
import { realpath } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import type { Plugin, PluginOptions } from '@opencode/plugin';
import type { CliError, CliResult, CompletionControl, ToolArguments } from '../shared/protocol.ts';

type Session = Awaited<ReturnType<Plugin.Context['session']['get']>>;
type SessionID = Parameters<Plugin.Context['session']['get']>[0]['sessionID'];
type Active = {project:string;enabled:boolean;mode:string;entryId:string;runId?:string};
type Metadata = Session['metadata'] & {[OWNER]?: Active};
type ToolContext = {sessionID:SessionID;signal?:AbortSignal};
type ToolInput = Partial<ToolArguments['complete'] & ToolArguments['start'] & ToolArguments['resume'] & ToolArguments['reject'] & ToolArguments['inspect'] & ToolArguments['rename']>;

const OWNER='unelos.opencode';
const fail=(code:string,message:string)=>Object.assign(new Error(message),{code});
const object=<P extends Record<string, unknown>>(properties={} as P,required:string[]=[])=>({type:'object' as const,properties,required,additionalProperties:false});
const string={type:'string' as const};
const identity=object({kind:{type:'string',enum:['task','human-gate','parallel']},runId:string,nodeId:string,taskId:string,activationId:string},['kind','runId','nodeId']);
const descriptions={next:'Read the selected Unelos execution package',start:'Start a fresh Unelos workflow and bind this session',resume:'Bind the explicitly selected unfinished run and immediately continue using core resume context',complete:'Complete the expected Unelos Task after inspection',approve:'Record explicit approval of the expected human gate',reject:'Reject the expected human gate with actionable feedback',status:'Read selected Unelos status',inspect:'Inspect a Unelos workflow or Node',rename:'Name the selected Unelos run'};

export const toolSchemas={
  next:object(),start:object({workflow:string,name:string},['workflow']),resume:object({runId:string},['runId']),
  complete:object({expected:identity,resultText:{...string,description:"the worker's final message, verbatim (only newly finished workers for parallel; write aggregates before the last submission; empty once all receipts exist)"},summary:{...string,description:'One-paragraph summary for main Tasks'},outcome:{...string,description:'main Tasks only'}},['expected']),
  approve:object({expected:identity},['expected']),reject:object({expected:identity,reason:string},['expected','reason']),
  status:object(),inspect:object({workflow:string,node:string},['workflow']),rename:object({name:string,runId:string},['name','runId']),
};

export function callCli(binary:string,project:string,argv:string[],input?:unknown,signal?:AbortSignal):Promise<CliResult> {
  const mutation=argv[0]==='rename'||argv[0]==='execution'&&['start','complete','approve','reject'].includes(argv[1]);
  const target=argv.includes('--run')?argv[argv.indexOf('--run')+1]:null;
  const guidance=target?`Reconcile with execution next --run ${target}; never blindly retry`:'List unfinished runs and explicitly reconcile; never blindly retry';
  return new Promise((resolve,reject)=>{
    const child=spawn(binary,['--project',project,...argv],{signal,timeout:10000,killSignal:'SIGKILL',stdio:['pipe','pipe','pipe']});
    let stdout='',stderr='',size=0;
    const uncertain=()=>fail(mutation?'mutation_uncertain':'cli_failed',mutation?`Mutation response unavailable; ${guidance}`:'Unelos CLI failed');
    for(const [stream,append] of [[child.stdout,(s:string)=>stdout+=s],[child.stderr,(s:string)=>stderr+=s]] as const) {
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
          try {const error=(JSON.parse(line) as Partial<CliError>|null)?.error;
            if(typeof error?.code==='string'&&typeof error.message==='string'){reject(fail(error.code,error.message));return;}
          }catch{ /* Non-JSON stderr lines are diagnostics, not structured CLI errors. */ }
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

function binaryOption(options:PluginOptions) {
  const binary=options?.binary??'unelos';
  if(typeof binary!=='string'||!binary.trim()||binary.includes('\0'))throw fail('invalid_binary','Specify one CLI executable');
  return binary;
}

function mapping() {
  return 'Native Unelos tools: '+Object.keys(toolSchemas).map(name=>'unelos_'+name).join(', ')+'.\n'
    +'Native worker: subagent(agent=nativeAgent, prompt=workerPrompt, sessionID for continuation).\n'
    +'Parallel workers: in one turn call subagent(agent=workerPrompts[i].nativeAgent, prompt=workerPrompts[i].prompt, background=true) for every entry before waiting; continue one worker with its sessionID.';
}

function nativePackage(value:CliResult) {
  return {...value,...(value.workerPrompt&&value.packet?.agent?{nativeAgent:'unelos-'+value.packet.agent}:{}),
    ...(value.workerPrompts?.length?{workerPrompts:value.workerPrompts.map(w=>({...w,nativeAgent:'unelos-'+w.agent}))}:{}),
    harnessInstructions:mapping()};
}

function bridge(binary:string,get:(id:SessionID)=>Promise<Session>,update:(id:SessionID,metadata:Metadata)=>Promise<unknown>,directory:(session:Session)=>string) {
  const changes=new Map<SessionID,Promise<unknown>>(); // Only serialize metadata check/write; CLI work never holds this queue.
  function change<T>(id:SessionID,fn:()=>Promise<T>) {
    const next=(changes.get(id)??Promise.resolve()).catch(()=>{}).then(fn);
    changes.set(id,next);
    return next.finally(()=>{if(changes.get(id)===next)changes.delete(id);});
  }
  async function session(id:SessionID) {
    const value=await get(id);if(value?.id!==id)throw fail('session_lookup_failed','Native session unavailable');
    if(value.parentID!=null)throw fail('child_controller','Workers cannot operate the Unelos controller');
    return value;
  }
  async function project(id:SessionID) {
    const value=await session(id);const active=(value.metadata as Metadata)?.[OWNER];
    if(!active?.enabled)throw fail('controller_inactive','Activate /unelos in the main session first');
    const path=await realpath(directory(value));
    if(active.project!==path)throw fail('project_changed','Project changed; reactivate /unelos');
    return {path,active,value};
  }
  async function current(id:SessionID,active:Active) {
    const state=await project(id);
    if(state.active.entryId!==active.entryId)throw fail('entry_superseded','Entry superseded; explicitly select a run again');
    return state.value;
  }
  return {
    async command(id:SessionID,text:string) {
      const words=text.trim().split(/\s+/).filter(Boolean);
      const keyword=words.length===1?words[0]:null;
      const entry=await change(id,async()=>{
        const value=await session(id);const path=await realpath(directory(value));const metadata={...value.metadata};
        if(keyword==='off'){delete metadata[OWNER];await update(id,metadata);return null;}
        const active={project:path,enabled:true,mode:keyword==='resume'?'resume':'new',entryId:randomUUID()};
        await update(id,{...metadata,[OWNER]:active});return {path,active};
      });
      if(!entry)return 'Unelos disabled for this session.';
      const {path,active}=entry;
       const result=nativePackage(await callCli(binary,path,['execution',active.mode==='new'?'new':'runs','--json']));
      await current(id,active);
      return JSON.stringify({...result,requestedAction:text});
    },
    async execute(name:string,input:ToolInput,context:ToolContext) {
      const {path,active}=await project(context.sessionID);let argv:(string|undefined)[],control:CompletionControl|undefined;
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
          argv=['execution',name,'--run',active.runId];control={expected:input.expected!,resultText:input.resultText??null,summary:input.summary??null,outcome:input.outcome??null,reason:input.reason??null};break;
        case 'status':argv=['status','--run',active.runId];break;
        case 'inspect':argv=['inspect',input.workflow,...(input.node?['--node',input.node]:[])];break;
        case 'rename':argv=['rename',input.name,'--run',active.runId];break;
        default:throw fail('invalid_tool','Unknown Unelos tool');
      }
      const result=await callCli(binary,path,[...argv,'--json'] as string[],control,context.signal);
      if(entry){
        try{
          if(typeof result.runId!=='string'||!result.runId.trim()||name==='resume'&&result.runId!==input.runId)throw fail('invalid_cli_json','CLI returned no valid selected run ID');
          await change(context.sessionID,async()=>{
            const value=await current(context.sessionID,active);
            if((value.metadata as Metadata)[OWNER]!.runId)throw fail('entry_superseded','Entry already bound; select a run again');
            await update(context.sessionID,{...value.metadata,[OWNER]:{...active,runId:result.runId as string}});
          });
        }catch(error){
          if(name==='start')throw fail('mutation_uncertain',`Run ${result.runId??'(unknown)'} may have been created but binding failed; explicitly reconcile with /unelos resume; never retry start`);
          if(error instanceof Error && 'code' in error && ['controller_inactive','project_changed','entry_superseded'].includes(String(error.code)))throw fail('entry_superseded','Resume entry superseded; select a run again');
          throw error;
        }
      }
      return nativePackage(result);
    },
  };
}

export default {
  id:OWNER,
  async setup(ctx) {
    const native=bridge(binaryOption(ctx.options),id=>ctx.session.get({sessionID:id}),
      (id,metadata)=>ctx.session.update({sessionID:id,metadata}),s=>s.location.directory);
    await ctx.tool.transform(editor=>{
      for(const [name,input] of Object.entries(toolSchemas))editor.add({name:'unelos_'+name,description:descriptions[name as keyof typeof descriptions],input,
        execute:async(args,context)=>({content:JSON.stringify(await native.execute(name,args as ToolInput,context))})});
    });
    await ctx.command.transform(editor=>editor.add({name:'unelos',description:'Activate the Unelos controller',
      execute:async({sessionID,prompt,delivery})=>{await ctx.session.prompt({...prompt,sessionID,delivery,text:await native.command(sessionID,prompt.text)});}}));
  },
} satisfies Plugin.Plugin;
