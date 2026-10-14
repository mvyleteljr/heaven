// Build the optional bridge from the inspected PR revision. No package install
// or sibling checkout mutation. Pass an extracted pinned archive or checkout.
import {readFileSync, existsSync} from 'node:fs';
import {installRuntime} from './onto-runtime-files.mjs';
import {resolve, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const revision='454af8cb5bf9d6bef725df6a3294a5e54be37894';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const source=resolve(process.argv[2] ?? '../onto');
if (Number(process.versions.node.split('.')[0])<24) throw new Error('Build with Node 24+');
const actual=existsSync(resolve(source,'.frontier-revision'))?readFileSync(resolve(source,'.frontier-revision'),'utf8').trim():execFileSync('git',['-C',source,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if (actual!==revision) throw new Error(`Expected Onto ${revision}, got ${actual}`);
const toolchain=resolve(process.env.ONTO_TOOLCHAIN_ROOT ?? resolve(root,'../onto'));
const require=createRequire(resolve(toolchain,'package.json'));
const output=resolve(root,'data/onto-runtime.cjs');
const packages=['ontolang','ontolang-runtime'];
const alias={};
for (const name of packages) {
  const dir=resolve(source,'packages',name);
  const pkg=JSON.parse(readFileSync(resolve(dir,'package.json'),'utf8'));
  for (const [key,value] of Object.entries(pkg.exports)) {
    const file=typeof value==='string'?value:value.default;
    if (file) alias[`@trigramco/${name}${key==='.'?'':key.slice(1)}`]=resolve(dir,file.replace('./dist/','./src/').replace(/\.js$/,'.ts'));
  }
}
const built=require('esbuild').buildSync({entryPoints:[resolve(root,'src/onto_host.mjs')],bundle:true,platform:'node',format:'cjs',target:'node24',alias,nodePaths:[resolve(toolchain,'node_modules'),resolve(toolchain,'packages/ontolang/node_modules')],outfile:output,write:false,logLevel:'warning'});
const bytes=built.outputFiles[0].contents;
const manifest={onto_revision:revision,bridge_sha256:createHash('sha256').update(bytes).digest('hex'),node:process.versions.node,esbuild:require('esbuild').version,toolchain_lock_sha256:createHash('sha256').update(readFileSync(resolve(toolchain,'pnpm-lock.yaml'))).digest('hex')};
installRuntime(output,bytes,manifest);
console.log(JSON.stringify(manifest));
