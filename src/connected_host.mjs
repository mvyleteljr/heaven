// Connected-world operations use the same pinned interpreter as legacy worlds.
import {parseProjectFiles} from '@trigramco/ontolang';
import {compileModel, initializeWorld, executeStory, tick} from '@trigramco/ontolang-runtime';
import {isDeepStrictEqual} from 'node:util';

export function connected(request) {
  const source = {...request.foundation};
  const owners = {}, storyOwners = {};
  const parts = {foundation: Object.values(request.foundation).join('\n'), ...request.parts};
  for (const [name, text] of Object.entries(parts)) {
    if (/^\s*import\s/m.test(text)) throw Error('Connected modules share vocabulary directly; imports are unsupported');
    if (name !== 'foundation') source[`module-${name}.onto`] = text;
    for (const match of text.matchAll(/^(?:object|actor)\s+([A-Za-z][A-Za-z0-9_]*)\b/gm)) {
      if (owners[match[1]]) throw Error(`Duplicate type declaration: ${match[1]}`);
      owners[match[1]] = name;
    }
  }
  const model = compileModel(parseProjectFiles('connected-institution', source));
  for (const name of model.entities.keys()) if (!owners[name]) throw Error(`Unowned type: ${name}`);
  for (const story of model.stories) {
    const matches = Object.entries(parts).filter(([,text]) => text.split('\n').some(line => line.trim() === story.text));
    if (matches.length !== 1 || storyOwners[story.text]) throw Error(`Ambiguous Story ownership: ${story.text}`);
    storyOwners[story.text] = matches[0][0];
    if (story.every !== undefined || story.external) throw Error('Use bounded host schedules; native every and external Stories are unsupported');
  }
  if (request.mode === 'compose') {
    const previousSource = {...request.foundation};
    for (const [name,text] of Object.entries(request.previous_parts ?? {})) previousSource[`module-${name}.onto`] = text;
    const previous = compileModel(parseProjectFiles('connected-institution', previousSource));
    const signature = rule => ({source:rule.source,ast:rule.ast,timing:rule.timing,attachment:rule.attachment});
    for (const rule of model.rules) {
      if (rule.attachment.kind === 'type' && owners[rule.attachment.typeName] !== request.module
        && !previous.rules.some(r=>isDeepStrictEqual(signature(r),signature(rule)))) throw Error('Publication introduced a rule on another module');
    }
  }
  const now = new Date(Date.parse(request.epoch) + request.elapsed_seconds * 1000).toISOString().slice(0,19);
  const initial = initializeWorld(model, now);
  const facts = structuredClone(request.snapshot ?? initial.snapshot);
  const existing = new Map(facts.instances.map(r => [r.id,r]));
  if (request.mode === 'compose') {
    for (const row of initial.snapshot.instances) if (!existing.has(row.id)) {
      if (owners[row.type] !== request.module) throw Error('Publication cannot seed another module');
      facts.instances.push(row);
    }
    for (const patch of request.migration ?? []) {
      const row = existing.get(patch.id);
      if (!row || owners[row.type] !== request.module) throw Error('Migration outside owned records');
      if (!patch.fields || typeof patch.fields !== 'object' || Array.isArray(patch.fields) || Object.keys(patch).some(k=>!['id','fields'].includes(k))) throw Error('Invalid migration patch');
      for (const key of Object.keys(patch.fields)) {
        const field = model.catalog(row.type).find(f=>f.name===key);
        if (!field || field.computed || field.type === 'link') throw Error('Migration requires declared stored fields');
      }
      Object.assign(row.fields, patch.fields);
    }
  }
  // Validate existing records against a new schema, including required fields.
  for (const row of facts.instances) {
    if (!model.entities.has(row.type)) throw Error(`Migration removed live type ${row.type}`);
    const fields = model.catalog(row.type);
    for (const [key, value] of Object.entries(row.fields)) {
      const field = fields.find(f => f.name === key);
      if (!field && key === 'name' && model.entities.get(row.type)?.kind === 'actor' && typeof value === 'string') continue;
      if (!field) throw Error(`Undeclared field ${row.type}.${key}`);
      if (field.computed || field.type === 'link') continue;
      if (['int','money','percent'].includes(field.type) && !Number.isSafeInteger(value)) throw Error(`Invalid integer ${row.id}.${key}`);
      if (field.type === 'float' && (typeof value !== 'number' || !Number.isFinite(value))) throw Error(`Invalid number ${row.id}.${key}`);
      if (field.type === 'bool' && typeof value !== 'boolean') throw Error(`Invalid boolean ${row.id}.${key}`);
      if (['string','timestamp','enum'].includes(field.type) && typeof value !== 'string') throw Error(`Invalid text ${row.id}.${key}`);
      if (field.type === 'timestamp' && !Number.isFinite(Date.parse(value))) throw Error(`Invalid timestamp ${row.id}.${key}`);
      if (field.enumValues?.length && !field.enumValues.includes(value)) throw Error(`Invalid enum ${row.id}.${key}`);
      if (field.type === 'ref' && value != null && !facts.instances.some(r => r.id === value && (r.type === field.refType || model.subtypesOf(field.refType).includes(r.type)))) throw Error(`Invalid reference ${row.id}.${key}`);
    }
    for (const field of fields) if (field.required && !field.computed && row.fields[field.name] == null) throw Error(`Migration must supply ${row.id}.${field.name}`);
  }
  let current = tick(model.ruleset, facts, now);
  if (request.mode === 'compose' && current.standing.length) throw Error(`Migration violates institution: ${JSON.stringify(current.standing)}`);
  let result;
  if (request.mode === 'execute') {
    const story = model.story(request.command.story);
    if (!story || storyOwners[story.text] !== request.module) throw Error('Story is outside module');
    if (story.actorType === 'system' && !request.host_event) throw Error('System Stories are host-only');
    const execution = executeStory(model, current.snapshot, {...request.command, actor: story.actorType === 'system' ? undefined : request.actor});
    const checked = tick(model.ruleset, execution.snapshot, now);
    const accepted = execution.committed && checked.standing.length === 0;
    result = {committed:accepted, key:execution.key, ...(accepted ? {created:execution.created} : {}), violations:[...execution.violations,...checked.standing]};
    if (accepted) current = checked;
  }
  // Publication may change derived values elsewhere, but never stored facts.
  if (request.mode === 'compose' && request.snapshot) {
    for (const old of request.snapshot.instances) if (owners[old.type] !== request.module) {
      const next = current.snapshot.instances.find(r => r.id === old.id);
      const stored = row => ({...row,fields:Object.fromEntries(Object.entries(row.fields).filter(([k]) => !model.catalog(row.type).some(f => f.name === k && (f.computed || f.type === 'link'))))});
      if (!next || !isDeepStrictEqual(stored(old), stored(next))) throw Error('Publication changed another module');
    }
  }
  return {snapshot:current.snapshot, standing:current.standing, result, owners, story_owners:storyOwners,
    stories:model.stories.map(s=>({story:s.text,creates:s.creates,object:s.primary?.type,participants:s.participants.map(p=>p.token),actor_type:s.actorType,actor:s.subject.type,system:s.actorType==='system'})),
    fields:Object.fromEntries([...model.entities.keys()].map(t=>[t,model.catalog(t)]))};
}
