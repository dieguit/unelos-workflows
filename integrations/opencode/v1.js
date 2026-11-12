import {createRequire} from 'node:module';
import {homedir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';

async function nativeTool(options) {
  if(options?.v1ToolModule){
    try{return (await import(pathToFileURL(options.v1ToolModule).href)).tool;}
    catch{throw Object.assign(new Error('V1 native tool SDK unavailable at configured v1ToolModule'),{code:'native_tool_sdk_unavailable'});}
  }
  const roots=[import.meta.url,pathToFileURL(join(process.env.OPENCODE_CONFIG_DIR??join(process.env.XDG_CONFIG_HOME??join(homedir(),'.config'),'opencode'),'package.json')).href];
  for(const root of roots){try{return (await import(pathToFileURL(createRequire(root).resolve('@opencode-ai/plugin/tool')).href)).tool;}catch{}}
  throw Object.assign(new Error('V1 native tool SDK unavailable; provide the harness-installed @opencode-ai/plugin/tool via v1ToolModule'),{code:'native_tool_sdk_unavailable'});
}

export async function registerV1(options,descriptions,execute) {
  const tool=await nativeTool(options);const z=tool.schema;
  const expected=z.object({kind:z.enum(['task','human-gate']),runId:z.string(),nodeId:z.string(),taskId:z.string().optional(),activationId:z.string().optional()});
  const schemas={next:{},start:{workflow:z.string(),name:z.string().optional()},resume:{runId:z.string()},complete:{expected,resultText:z.string().describe("the worker's final message, verbatim (all workers' final messages for parallel)").optional(),summary:z.string().describe('One-paragraph summary for main Tasks').optional(),outcome:z.string().describe('main Tasks only').optional()},approve:{expected},reject:{expected,reason:z.string()},status:{},inspect:{workflow:z.string(),node:z.string().optional()},rename:{name:z.string(),runId:z.string()}};
  return Object.fromEntries(Object.entries(schemas).map(([name,args])=>['unelos_'+name,tool({description:descriptions[name],args,
    execute:async(input,context)=>JSON.stringify(await execute(name,input,context))})]));
}
