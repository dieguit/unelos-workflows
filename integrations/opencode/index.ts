import { spawn } from 'node:child_process';
import { realpath } from 'node:fs/promises';
import type { Plugin, PluginOptions } from '@opencode/plugin';
import type { CliError, CliResult } from '../shared/protocol.ts';

const fail=(code:string,message:string)=>Object.assign(new Error(message),{code});
const TOOLS=new Set(['next','start','resume','complete','approve','reject','status','inspect','rename'].map(name=>'unelos_'+name));

export function callCli(binary:string,project:string,argv:string[]):Promise<CliResult> {
  return new Promise((resolve,reject)=>{
    const child=spawn(binary,['--project',project,...argv],{timeout:10000,killSignal:'SIGKILL',stdio:['ignore','pipe','pipe']});
    let stdout='',stderr='',size=0;
    for(const [stream,append] of [[child.stdout,(s:string)=>stdout+=s],[child.stderr,(s:string)=>stderr+=s]] as const) {
      stream.setEncoding('utf8');stream.on('data',s=>{
        size+=Buffer.byteLength(s);
        if(size>16*1024*1024){child.kill('SIGKILL');reject(fail('cli_failed','Unelos response too large'));}else append(s);
      });
    }
    child.once('error',()=>reject(fail('cli_failed','Unelos CLI failed')));
    child.once('close',code=>{
      if(code!==0){
        for(const line of stderr.trim().split('\n').reverse()){
          try {
            const error=(JSON.parse(line) as Partial<CliError>|null)?.error;
            if(typeof error?.code==='string'&&typeof error.message==='string'){reject(fail(error.code,error.message));return;}
          }catch{}
        }
        reject(fail('cli_failed','Unelos CLI failed'));return;
      }
      try {
        const value=JSON.parse(stdout);
        if(!value||typeof value!=='object'||Array.isArray(value))throw new Error();
        resolve(value);
      }catch{reject(fail('invalid_cli_json','Unelos returned invalid JSON'));}
    });
  });
}

function binaryOption(options:PluginOptions) {
  const binary=options?.binary??'unelos';
  if(typeof binary!=='string'||!binary.trim()||binary.includes('\0'))throw fail('invalid_binary','Specify one CLI executable');
  return binary;
}

export default {
  id:'unelos.opencode',
  async setup(ctx) {
    const binary=binaryOption(ctx.options);
    await ctx.mcp.transform(editor=>editor.set('unelos',{type:'local',command:[binary,'mcp']}));
    await ctx.tool.hook('execute.before',async event=>{
      if(!TOOLS.has(event.tool))return;
      const session=await ctx.session.get({sessionID:event.sessionID});
      if(!session||session.id!==event.sessionID)throw fail('session_lookup_failed','Native session unavailable');
      if(!event.input||typeof event.input!=='object'||Array.isArray(event.input))throw fail('context_missing','Tool input must be an object');
      (event.input as Record<string,unknown>).unelos_context={harness:'opencode',sessionId:event.sessionID,
        project:await realpath(session.location.directory),role:session.parentID!=null?'worker':'controller'};
    });
    await ctx.command.transform(editor=>editor.add({name:'unelos',description:'Activate the Unelos controller',
      execute:async({sessionID,prompt,delivery})=>{
        const session=await ctx.session.get({sessionID});
        if(!session||session.id!==sessionID)throw fail('session_lookup_failed','Native session unavailable');
        if(session.parentID!=null)throw fail('child_controller','Workers cannot activate the Unelos controller');
        const project=await realpath(session.location.directory);
        const result=await callCli(binary,project,['session','activate','--harness','opencode','--session',sessionID,'--request',prompt.text,'--json']);
        await ctx.session.prompt({...prompt,sessionID,delivery,text:JSON.stringify({...result,requestedAction:prompt.text})});
      }}));
  },
} satisfies Plugin.Plugin;
