// Read-only compatibility spike. Uses an existing Onto checkout; no network,
// generated executable code, package installation, or writes in that checkout.
import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(process.env.ONTO_ROOT ?? resolve(here, '../../onto'));
assert.ok(Number(process.versions.node.split('.')[0]) >= 24, 'Onto requires Node 24+');
const language = await import(pathToFileURL(resolve(root, 'packages/ontolang/dist/index.js')));
const source = readFileSync(resolve(here, '../examples/onto/contracts.onto'), 'utf8');
const parse = text => language.parseProjectFiles('harbor-contracts', {'contracts.onto': text});
const model = parse(source);
language.assertWorkflowsReady(model);
const printed = language.printProject(model);
const reopened = language.parseProjectFiles('harbor-contracts', printed);
assert.deepEqual(language.printProject(reopened), printed, 'Canonical source must round-trip');
assert.throws(() => parse(source.replace('priceCents: int', 'priceCents: string')), /numeric|number|compar|type/i);
assert.throws(() => parse(source.replace('sets Agreement.status to "accepted"', 'sets Agreement.status to "invented"')));
assert.throws(() => language.assertWorkflowsReady(parse(source.replace('paid @final', 'paid'))));

// The portable package is the language, not an executor. Use the separate
// reference runtime explicitly and pin its revision in the report.
const requireOnto = createRequire(resolve(root, 'package.json'));
const temporary = mkdtempSync(resolve(tmpdir(), 'frontier-onto-'));
let runtime;
try {
  requireOnto('esbuild').buildSync({entryPoints:[resolve(root,'rule-runtime/index.ts')],bundle:true,platform:'node',format:'cjs',outfile:resolve(temporary,'runtime.cjs'),logLevel:'silent'});
  runtime = requireOnto(resolve(temporary,'runtime.cjs'));
} finally { rmSync(temporary,{recursive:true,force:true}); }
const { compileModel, replayExecutionLog } = runtime;
const compiled = compileModel(reopened);
const log = JSON.parse(readFileSync(resolve(here, '../examples/onto/contracts.trace.json'), 'utf8'));
const report = replayExecutionLog(compiled, log);
assert.deepEqual(report.failures, []);
assert.equal(report.ok, true);
console.log(JSON.stringify({
  onto_revision: execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], {encoding:'utf8'}).trim(),
  source_sha256: createHash('sha256').update(source).digest('hex'),
  round_trip: true,
  rejected_model_mutations: 3,
  trace_steps: log.steps.length,
  refusals_expected: log.steps.filter(s => s.expect?.committed === false).length,
  replay_ok: report.ok,
  final_agreement: report.snapshot.instances.find(i => i.id === 'agreement-1')?.fields,
  boundary: 'Reference workflow execution only. No ledger, hosting verification, demand generation, or Frontier runtime integration.'
}, null, 2));
