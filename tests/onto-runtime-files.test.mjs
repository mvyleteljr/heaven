import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {installRuntime} from '../scripts/onto-runtime-files.mjs';

const manifest = bytes => ({bridge_sha256: createHash('sha256').update(bytes).digest('hex'), onto_revision: 'test'});

test('runtime upgrades archive both versions and manifests before replacing the default', () => {
  const directory = mkdtempSync(join(tmpdir(), 'frontier-runtime-'));
  try {
    const output = join(directory, 'onto-runtime.cjs');
    const old = Buffer.from('old trusted runtime');
    const next = Buffer.from('new trusted runtime');
    installRuntime(output, old, manifest(old));
    installRuntime(output, next, manifest(next));
    for (const bytes of [old, next]) {
      const hash = manifest(bytes).bridge_sha256;
      assert.deepEqual(readFileSync(join(directory, 'onto-runtimes', `${hash}.cjs`)), bytes);
      assert.deepEqual(JSON.parse(readFileSync(join(directory, 'onto-runtimes', `${hash}.json`))), manifest(bytes));
    }
    assert.deepEqual(readFileSync(output), next);
    assert.deepEqual(JSON.parse(readFileSync(join(directory, 'onto-runtime.json'))), manifest(next));
    assert.throws(() => installRuntime(output, old, manifest(next)), /does not match/);
    assert.deepEqual(readFileSync(output), next);
    writeFileSync(join(directory, 'onto-runtimes', `${manifest(next).bridge_sha256}.cjs`), 'corrupt');
    assert.throws(() => installRuntime(output, old, manifest(old)), /Corrupt archived/);
    assert.deepEqual(readFileSync(output), next);
  } finally {
    rmSync(directory, {recursive: true, force: true});
  }
});
