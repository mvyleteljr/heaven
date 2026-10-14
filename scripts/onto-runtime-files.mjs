// Content-addressed runtime archives preserve saved runs across bridge builds.
import {existsSync, mkdirSync, readFileSync, renameSync, unlinkSync, writeFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {createHash, randomUUID} from 'node:crypto';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
function atomicWrite(path, bytes) {
  const temporary = `${path}.${randomUUID()}.tmp`;
  try {
    writeFileSync(temporary, bytes, {flag: 'wx'});
    renameSync(temporary, path);
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
}

function archive(directory, bytes, manifestBytes) {
  const hash = digest(bytes);
  const path = join(directory, `${hash}.cjs`);
  if (existsSync(path)) {
    if (digest(readFileSync(path)) !== hash) throw new Error(`Corrupt archived runtime: ${path}`);
  } else {
    atomicWrite(path, bytes);
  }
  if (manifestBytes) {
    const manifest = JSON.parse(manifestBytes.toString());
    if (manifest.bridge_sha256 !== hash) throw new Error('Runtime manifest does not match bundle');
    const manifestPath = join(directory, `${hash}.json`);
    if (existsSync(manifestPath)) {
      if (JSON.parse(readFileSync(manifestPath)).bridge_sha256 !== hash) throw new Error(`Corrupt archived manifest: ${manifestPath}`);
    } else {
      atomicWrite(manifestPath, manifestBytes);
    }
  }
}

export function installRuntime(output, bytes, manifest) {
  if (manifest.bridge_sha256 !== digest(bytes)) throw new Error('New runtime manifest does not match bundle');
  const directory = dirname(output);
  const archiveDirectory = join(directory, 'onto-runtimes');
  const manifestPath = output.replace(/\.cjs$/, '.json');
  const lock = `${output}.build.lock`;
  mkdirSync(archiveDirectory, {recursive: true});
  // Serialize default bundle/manifest replacement; generation itself is in memory.
  writeFileSync(lock, '', {flag: 'wx'});
  try {
    if (existsSync(output)) {
      archive(archiveDirectory, readFileSync(output), existsSync(manifestPath) ? readFileSync(manifestPath) : undefined);
    }
    const manifestBytes = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`);
    archive(archiveDirectory, bytes, manifestBytes);
    atomicWrite(output, bytes);
    atomicWrite(manifestPath, manifestBytes);
  } finally {
    unlinkSync(lock);
  }
}
