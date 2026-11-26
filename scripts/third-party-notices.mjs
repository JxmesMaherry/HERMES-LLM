// Generate redistribution notices from installed production npm packages and the locked macOS Rust graph.
// Run after npm ci and cargo fetch/build. No secrets or machine paths enter the output.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const metadata=JSON.parse(execFileSync('cargo',['metadata','--locked','--offline','--manifest-path','apps/desktop/src-tauri/Cargo.toml','--filter-platform','aarch64-apple-darwin','--format-version','1'],{cwd:root,encoding:'utf8',maxBuffer:20*1024*1024}));
const byId=new Map(metadata.packages.map(p=>[p.id,p]));const nodes=new Map(metadata.resolve.nodes.map(n=>[n.id,n]));const selected=new Set();
function visit(id){if(selected.has(id))return;selected.add(id);for(const d of nodes.get(id)?.deps??[]){if(d.dep_kinds.some(k=>k.kind!=='dev'))visit(d.pkg);}}
visit(metadata.resolve.root);
const packages=[...selected].map(id=>byId.get(id)).filter(p=>p?.source).map(p=>({name:`Rust: ${p.name} ${p.version}`,license:p.license,dir:path.dirname(p.manifest_path),licenseFile:p.license_file,fallback:`${p.name}-${p.version}`}));
const npmPaths=execFileSync('npm',['ls','--omit=dev','--parseable','--all','--workspace','@hermes/desktop'],{cwd:root,encoding:'utf8'}).trim().split('\n');
for(const dir of [...new Set(npmPaths)]){const p=JSON.parse(fs.readFileSync(path.join(dir,'package.json'),'utf8'));if(p.private)continue;packages.push({name:`npm: ${p.name} ${p.version}`,license:typeof p.license==='string'?p.license:JSON.stringify(p.license??p.licenses),dir});}
const missing=[];const sections=[];
for(const p of packages.sort((a,b)=>a.name.localeCompare(b.name))){
 const files=fs.readdirSync(p.dir,{withFileTypes:true}).filter(f=>f.isFile()&&/^(licen[cs]e|copying|copyright|notice)([.-].*)?$/i.test(f.name)).map(f=>f.name);
 if(p.licenseFile&&!files.includes(p.licenseFile))files.push(p.licenseFile);
 const legal=path.join(p.dir,'LICENSES');if(fs.existsSync(legal)&&fs.statSync(legal).isDirectory())for(const f of fs.readdirSync(legal)){if(fs.statSync(path.join(legal,f)).isFile())files.push(`LICENSES/${f}`);}
 const fallback=p.fallback?path.join(root,'docs/upstream-licenses',p.fallback):null;
 const fallbackFiles=fallback&&fs.existsSync(fallback)?fs.readdirSync(fallback):[];
 if(!files.length&&!fallbackFiles.length)missing.push(p.name);
 const upstream=fallbackFiles.map(f=>`--- Upstream ${f} ---\n${fs.readFileSync(path.join(fallback,f),'utf8')}`).join('\n\n');
 sections.push(`${'='.repeat(78)}\n${p.name}\nDeclared licence: ${p.license??'See upstream package'}\n\n${files.map(f=>`--- ${f} ---\n${fs.readFileSync(path.join(p.dir,f),'utf8')}`).join('\n\n')||upstream||'No standalone licence file shipped in this package. Consult its declared upstream licence.'}`);
}
const intro='HERMES-LLM — THIRD-PARTY NOTICES\n\nGenerated from the locked Apple Silicon Rust production/build graph and production npm packages.\nThese notices retain upstream terms and do not relicense upstream code.\nOpen WebUI, Docker Desktop, Ollama and model weights are separately obtained; see THIRD-PARTY.md.\n\n';
fs.writeFileSync(path.join(root,'docs/THIRD-PARTY-LICENSES.txt'),intro+sections.join('\n\n')+'\n');
console.log(`Wrote notices for ${packages.length} packages. ${missing.length} package(s) supplied no standalone licence file.`);
if(missing.length)console.log(missing.join('\n'));
