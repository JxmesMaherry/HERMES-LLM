import fs from 'node:fs';
const root=JSON.parse(fs.readFileSync(new URL('../package.json',import.meta.url)));
const desktop=JSON.parse(fs.readFileSync(new URL('../apps/desktop/package.json',import.meta.url)));
const tauri=JSON.parse(fs.readFileSync(new URL('../apps/desktop/src-tauri/tauri.conf.json',import.meta.url)));
const cargo=fs.readFileSync(new URL('../apps/desktop/src-tauri/Cargo.toml',import.meta.url),'utf8').match(/^version = "([^"]+)"/m)?.[1];
if([desktop.version,tauri.version,cargo].some(v=>v!==root.version))throw new Error('Update root, desktop, Tauri and Cargo package versions together.');
if(process.argv[2]?.startsWith('v')&&process.argv[2]!==`v${root.version}`)throw new Error('Git tag must match package version.');
console.log(`Version consistent: ${root.version}`);
