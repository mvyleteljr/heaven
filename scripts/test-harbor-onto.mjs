// Deterministic model acceptance suite. No LLM, credentials, network, or paid calls.
// Run after scripts/build-onto.mjs using Node 24+.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {resolve, dirname} from 'node:path';
import {fileURLToPath} from 'node:url';

assert.ok(Number(process.versions.node.split('.')[0]) >= 24, 'Onto requires Node 24+');
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const bridge = resolve(process.env.ONTO_RUNTIME ?? resolve(root, 'data/onto-runtime.cjs'));
const epoch = '2026-01-01T00:00:00Z';
const DAY = 86400;
const harbor = 'Business:Harbor Works';
const quay = 'Business:Quay Hall';
const verifier = 'Verifier:Service verifier';
const iso = seconds => new Date(Date.parse(epoch) + seconds * 1000).toISOString().slice(0, 19);
const original = {'harbor-works.onto': readFileSync(resolve(root, 'examples/onto/harbor-works.onto'), 'utf8')};
let source = original;
let snapshot;
let accepted = 0;
let refused = 0;
let sequence = 0;
function bridgeRequest(operation, extra = {}, actor = harbor, at = 0) {
  const request = {operation, source, actor, epoch, elapsed_seconds: at, ...extra};
  let output;
  try {
    output = execFileSync(process.execPath, [bridge], {input: JSON.stringify(request), encoding: 'utf8'});
  } catch (error) {
    throw new Error(`Bridge failed: ${error.stdout?.toString() || error.message}`, {cause: error});
  }
  const response = JSON.parse(output);
  assert.equal(response.error, undefined, response.error);
  return response;
}
const initial = bridgeRequest('initialize');
source = initial.source;
const reopened = bridgeRequest('initialize');
assert.deepEqual(reopened.source, source, 'Print/reopen must preserve canonical source');
assert.deepEqual(reopened.snapshot, initial.snapshot, 'Print/reopen must preserve initialized model');
snapshot = reopened.snapshot;
const field = (id, name) => snapshot.instances.find(record => record.id === id)?.fields[name];
const byType = type => snapshot.instances.filter(record => record.type === type);
const cashTotal = () => byType('Firm').reduce((sum, record) => sum + record.fields.balance, 0);
const openingTotal = cashTotal();
assert.equal(byType('Firm').length, 14);
assert.equal(field('Firm:harbor', 'balance'), 500000);
assert.equal(field('Firm:harbor', 'capacity'), 12);
assert.equal(field('Firm:association', 'freeCash'), 20000);
assert.equal(openingTotal, 2280000);

function execute(story, {actor = harbor, at = 0, ok = true, ...command} = {}) {
  const before = structuredClone(snapshot.instances);
  const response = bridgeRequest('execute', {
    snapshot,
    command: {story, key: `check-${++sequence}`, ...command},
  }, actor, at);
  assert.equal(response.result.committed, ok,
    `${story}: expected committed=${ok}; ${JSON.stringify(response.result)}`);
  if (ok) accepted++; else {
    refused++;
    assert.deepEqual(response.snapshot.instances, before, 'Refused action must preserve domain state');
  }
  snapshot = response.snapshot;
  assert.equal(cashTotal(), openingTotal, 'Every transition must conserve district cash');
  assert.equal(field('Calendar:month', 'cashTotal'), openingTotal);
  return response;
}
const propose = (name, overrides = {}, options = {}) => execute('Business proposes Agreement to Firm', {
  mint: name, participants: {Firm: 'Firm:quay'},
  fields: {name, price: 10000, units: 1, capability: 'directory', deadline: iso(9 * DAY), createdAt: iso(options.at ?? 0), ...overrides},
  ...options,
});
propose('negative-price', {price: -1}, {ok: false});
propose('oversized-work', {units: 13}, {ok: false});
propose('unaffordable', {price: 40001}, {ok: false});
propose('too-soon', {deadline: iso(DAY)}, {ok: false});
propose('past-month', {deadline: iso(31 * DAY)}, {ok: false});
propose('self-purchase', {}, {participants: {Firm: 'Firm:harbor'}, ok: false});
propose('another-seller', {}, {actor: quay, ok: false});
propose('agreement-main', {status: 'paid', seller: 'Firm:festival'});
assert.equal(field('agreement-main', 'status'), 'proposed', 'Birth status is authored, not caller-controlled');
assert.equal(field('agreement-main', 'seller'), 'Firm:harbor', 'Seller identity is authored');
assert.equal(field('Firm:quay', 'reserved'), 10000);
assert.equal(field('Firm:harbor', 'committedUnits'), 1);
execute('Business authorizes Agreement', {object: 'agreement-main', at: DAY, ok: false});
execute('Business authorizes Agreement', {actor: 'Business:Anchor Cafe', object: 'agreement-main', at: DAY, ok: false});
execute('Business authorizes Agreement', {actor: quay, object: 'agreement-main', ok: false});
execute('Business pays Agreement', {actor: quay, object: 'agreement-main', ok: false});
execute('Business authorizes Agreement', {actor: quay, object: 'agreement-main', at: DAY});
execute('Business accepts Agreement', {actor: quay, object: 'agreement-main', at: DAY, ok: false});
execute('Business registers Service with Firm', {
  mint: 'service-main', at: DAY, participants: {Firm: 'Firm:printer'},
  fields: {name: 'directory', entrypoint: 'service.sh', capability: 'directory', createdAt: iso(DAY), available: true, portable: true},
});
assert.equal(field('service-main', 'available'), false);
assert.equal(field('service-main', 'portable'), false);
assert.equal(field('Firm:harbor', 'balance'), 490000);
assert.equal(field('Firm:printer', 'balance'), 100000);
const deliver = (ok = true, at = DAY) => execute('Business delivers Agreement with Service', {
  object: 'agreement-main', participants: {Service: 'service-main'}, fields: {deliveredAt: iso(at)}, at, ok,
});
deliver(false);
execute('Verifier verifies Service', {object: 'service-main', fields: {checkedAt: iso(DAY)}, at: DAY, ok: false});
execute('Verifier verifies Service', {actor: verifier, object: 'service-main', fields: {checkedAt: iso(DAY)}, at: DAY});
deliver();
assert.equal(field('agreement-main', 'status'), 'delivered');
assert.equal(field('Firm:harbor', 'sales'), 0, 'Delivery is not acceptance or payment');
execute('Business pays Agreement', {actor: quay, object: 'agreement-main', at: DAY, ok: false});
execute('Business accepts Agreement', {actor: quay, object: 'agreement-main', at: DAY, ok: false});
execute('Verifier faults Service', {actor: verifier, object: 'service-main', fields: {checkedAt: iso(2 * DAY)}, at: 2 * DAY});
execute('Business accepts Agreement', {actor: quay, object: 'agreement-main', at: 2 * DAY, ok: false});
execute('Business returns Agreement', {actor: quay, object: 'agreement-main', at: 2 * DAY});
execute('Verifier verifies Service', {actor: verifier, object: 'service-main', fields: {checkedAt: iso(2 * DAY)}, at: 2 * DAY});
deliver(true, 2 * DAY);
execute('Verifier verifies Service', {actor: verifier, object: 'service-main', fields: {checkedAt: iso(3 * DAY)}, at: 3 * DAY});
execute('Verifier certifies Service', {actor: verifier, object: 'service-main', fields: {checkedAt: iso(3 * DAY)}, at: 3 * DAY});
execute('Business accepts Agreement', {object: 'agreement-main', at: 3 * DAY, ok: false});
execute('Business accepts Agreement', {actor: quay, object: 'agreement-main', at: 3 * DAY});
assert.equal(field('Firm:harbor', 'sales'), 0);
execute('Business pays Agreement', {object: 'agreement-main', at: 3 * DAY, ok: false});
execute('Business pays Agreement', {actor: quay, object: 'agreement-main', at: 3 * DAY});
assert.equal(field('Firm:harbor', 'balance'), 500000);
assert.equal(field('Firm:quay', 'balance'), 110000);
assert.equal(field('Firm:quay', 'reserved'), 0);
execute('Business pays Agreement', {actor: quay, object: 'agreement-main', at: 3 * DAY, ok: false});
execute('Business rejects Agreement', {actor: quay, object: 'agreement-main', at: 3 * DAY, ok: false});
propose('buyer-reservation', {price: 20000}, {at: 3 * DAY});
propose('double-reservation', {price: 10001}, {at: 3 * DAY, ok: false});
execute('Business rejects Agreement', {actor: quay, object: 'buyer-reservation', at: 4 * DAY});
propose('unsupported-demand', {capability: 'delivery'}, {at: 4 * DAY});
execute('Business authorizes Agreement', {actor: quay, object: 'unsupported-demand', at: 5 * DAY, ok: false});
execute('Business rejects Agreement', {actor: quay, object: 'unsupported-demand', at: 5 * DAY});
propose('capacity-reservation', {units: 11, capability: 'coordination'}, {participants: {Firm: 'Firm:festival'}, at: 5 * DAY});
propose('seller-overcommit', {}, {at: 5 * DAY, ok: false});
execute('Business authorizes Agreement', {actor: 'Business:Festival Committee', object: 'capacity-reservation', at: 6 * DAY, ok: false});
execute('Business rejects Agreement', {actor: 'Business:Festival Committee', object: 'capacity-reservation', at: 6 * DAY});
propose('buyer-overcommit', {units: 2}, {at: 6 * DAY});
execute('Business authorizes Agreement', {actor: quay, object: 'buyer-overcommit', at: 7 * DAY, ok: false});
execute('Business rejects Agreement', {actor: quay, object: 'buyer-overcommit', at: 7 * DAY});
execute('Business completes CheckIn', {object: 'CheckIn:day-7', at: 7 * DAY, ok: false});
execute('Business completes CheckIn', {actor: 'Business:District Association', object: 'CheckIn:day-7', at: 6 * DAY, ok: false});
execute('Business completes CheckIn', {actor: 'Business:District Association', object: 'CheckIn:day-7', at: 7 * DAY});
execute('Business requests BackupPilot after CheckIn from Firm', {
  actor: 'Business:District Association', mint: 'backup-pilot', at: 7 * DAY,
  participants: {CheckIn: 'CheckIn:day-7', Firm: 'Firm:delivery'}, fields: {name: 'offline-roster', createdAt: iso(7 * DAY)},
});
assert.equal(field('Firm:association', 'balance'), 55000);
assert.equal(field('Firm:association', 'contingency'), 15000);
assert.equal(field('Firm:association', 'freeCash'), 20000);
assert.equal(field('Firm:delivery', 'balance'), 245000);
assert.equal(field('Firm:delivery', 'occupiedUnits'), 2);
execute('Business requests BackupPilot after CheckIn from Firm', {
  actor: 'Business:District Association', mint: 'duplicate-backup', at: 7 * DAY, ok: false,
  participants: {CheckIn: 'CheckIn:day-7', Firm: 'Firm:delivery'}, fields: {name: 'duplicate-roster', createdAt: iso(7 * DAY)},
});
propose('procurement-ceiling', {price: 20001}, {participants: {Firm: 'Firm:printer'}, at: 7 * DAY});
execute('Business authorizes Agreement', {actor: 'Business:Pier Print', object: 'procurement-ceiling', at: 8 * DAY, ok: false});
execute('Business rejects Agreement', {actor: 'Business:Pier Print', object: 'procurement-ceiling', at: 8 * DAY});
propose('closed-period', {deadline: iso(35 * DAY)}, {at: 30 * DAY, ok: false});
const final = bridgeRequest('inspect', {snapshot}, harbor, 30 * DAY);
assert.deepEqual(final.standing, []);
console.log(JSON.stringify({
  source: 'examples/onto/harbor-works.onto', round_trip: true,
  firms: byType('Firm').length, stories: initial.stories.length,
  accepted_steps: accepted, refused_steps: refused,
  conserved_cash_cents: cashTotal(), harbor_cash_cents: field('Firm:harbor', 'balance'),
  paid_agreements: byType('Agreement').filter(record => record.fields.status === 'paid').length,
  backup_pilots: byType('BackupPilot').length,
  boundary: 'Direct Onto semantics. Host integration separately enforces trusted identity, protected fields, executable service probes and exact-second schedules.',
}, null, 2));
