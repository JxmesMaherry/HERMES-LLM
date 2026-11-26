// Explicit live acceptance runner: pulls official image if needed, uploads ONLY the public fixture.
// Requires Docker Desktop and native Ollama with qwen3:4b-instruct already installed.
// Never removes a volume, personal file, model or unrelated container.
import {spawnSync} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {mkdtempSync,writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join,dirname,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
if(!process.argv.includes('--consent'))throw new Error('Run node scripts/integration.mjs --consent to approve downloading the official container and uploading the public test fixture locally.');
const port=Number(process.env.HERMES_TEST_PORT??13000);if(!Number.isInteger(port)||port<1024||port>65535)throw new Error('Invalid test port');
const project=`hermes-acceptance-${randomBytes(4).toString('hex')}`;
const dir=mkdtempSync(join(tmpdir(),'hermes-acceptance-'));const override=join(dir,'compose.override.yaml');
writeFileSync(override,`volumes:\n  open-webui-data:\n    name: ${project}-data\n`,{mode:0o600});
const env={...process.env,HERMES_PORT:String(port),HERMES_HF_OFFLINE:'1',WEBUI_SECRET_KEY:randomBytes(32).toString('hex')};
const args=['compose','--project-name',project,'--file',join(root,'infra/docker/compose.yaml'),'--file',override];
function docker(...more){const r=spawnSync('docker',[...args,...more],{env,stdio:'inherit',cwd:root});if(r.status!==0)throw new Error(`Docker ${more[0]} failed`);}
const base=`http://127.0.0.1:${port}`;
const ollama=await fetch('http://127.0.0.1:11434/api/tags').then(r=>r.json());if(!ollama.models?.some(m=>m.name==='qwen3:4b-instruct'))throw new Error('Install/select qwen3:4b-instruct before running this explicit live test. No model is downloaded automatically.');
console.log(`Isolated project ${project}; retained volume ${project}-data. The embedding model must be cached in the official image (offline mode).`);
let started=false;
try{
  docker('up','-d');started=true;
  let ready=false;for(let i=0;i<180;i++){try{ready=(await fetch(`${base}/health`,{signal:AbortSignal.timeout(3000)})).ok;}catch{}if(ready)break;await new Promise(r=>setTimeout(r,2000));}
  if(!ready)throw new Error('Test backend did not become ready in 6 minutes. Inspect its logs.');
  async function post(path,body,token){const r=await fetch(base+path,{method:'POST',headers:{'Content-Type':'application/json',...(token?{Authorization:`Bearer ${token}`}:{})},body:JSON.stringify(body)});if(!r.ok)throw new Error(`Local API ${path} failed (${r.status})`);return r.json();}
  const auth=await post('/api/v1/auths/signup',{name:'HERMES acceptance',email:`${project}@example.invalid`,password:randomBytes(32).toString('hex')});
  const kb=await post('/api/v1/knowledge/create',{name:'HERMES public acceptance fixture',description:'Only invented test facts'},auth.token);
  const result=spawnSync('cargo',['test','--locked','--manifest-path','apps/desktop/src-tauri/Cargo.toml','live_backend_retrieval_and_incremental_sync','--','--ignored','--nocapture'],{env:{...process.env,HERMES_TEST_PORT:String(port),HERMES_TEST_TOKEN:auth.token,HERMES_TEST_KB:kb.id},cwd:root,stdio:'inherit'});
  if(result.status!==0)throw new Error('Native live acceptance test failed');
  console.log('Live acceptance passed. No credentials were printed or saved to configuration.');
}finally{if(started)docker('stop');console.log(`Test data preserved in Docker volume ${project}-data. No volume was removed.`);}
