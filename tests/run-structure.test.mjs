import test from "node:test";
import assert from "node:assert/strict";
import { worldTree, findDomain, packDomain, renderStructure } from "../web/run-structure.js";
import { carouselWindow, previewMarkup } from "../web/run-carousel.js";

const institution = (balance) => ({ snapshot: { instances: [
  { id: "Account:Operating", type: "Account", fields: { name: "Operating", balance } },
  { id: "Account:Reserve", type: "Account", fields: { name: "Reserve", balance: 5000 } },
] }, stories: [{ id: "transfer", story: "Clerk transfers money" }] });
const world = { ontology: { ...institution(10000), modules: { treasury: institution(3000) } } };

test("domain identity survives fact changes and distinguishes module-local records", () => {
  const root = worldTree(world);
  const before = root.children[0].children[0].children[0];
  const changed = structuredClone(world);
  changed.ontology.snapshot.instances[0].fields.balance = 7500;
  const after = findDomain(worldTree(changed), before.id);
  assert.equal(after.fields.balance, 7500);
  assert.equal(root.children[1].children[0].children[0].fields.balance, 3000);
  assert.notEqual(before.id, root.children[1].children[0].children[0].id);
  assert.equal(root.count, 6);
  delete changed.ontology.modules.treasury;
  assert.equal(findDomain(worldTree(changed), root.children[1].id), null);
  assert.equal(findDomain(root, "world/missing"), null);
});

test("nested circle packing encloses children without overlapping siblings", () => {
  const layout = packDomain(worldTree(world));
  for (const parent of layout.descendants()) {
    assert.ok(Number.isFinite(parent.r) && parent.r > 0);
    const children = parent.children ?? [];
    for (const child of children) assert.ok(Math.hypot(child.x - parent.x, child.y - parent.y) + child.r <= parent.r + 1e-6);
    for (let i = 0; i < children.length; i++) for (let j = i + 1; j < children.length; j++) {
      const a = children[i], b = children[j];
      assert.ok(Math.hypot(a.x - b.x, a.y - b.y) >= a.r + b.r - 1e-6);
    }
  }
});

test("structure navigation inspects saved fields and safely falls back when a domain disappears", () => {
  const root = worldTree(world), record = root.children[1].children[0].children[0];
  const html = renderStructure(world, "domains", record.id);
  assert.match(html, /aria-current="location">Operating/);
  assert.match(html, /<dd>3000<\/dd>/);
  assert.match(html, /data-world-focus="world"/);
  assert.match(renderStructure({}, "domains", record.id), /aria-current="location">World/);
  assert.match(renderStructure(world, "tree"), /Clerk transfers money/);
  const unsafe = { tools: { public_resources: { '<img onerror="bad()">': { title: '<script>bad()</script>' } } } };
  for (const view of ["tree", "domains"]) {
    const rendered = renderStructure(unsafe, view);
    assert.doesNotMatch(rendered, /<script>|<img/);
    assert.match(rendered, /&lt;script&gt;/);
  }
});

test("carousel uses adjacent recorded events with stable identities and no fictional future", () => {
  const frames = [1, 7, 10, 14].map((seq) => ({ event: { seq, kind: "outcome", summary: `Event ${seq}` } }));
  const before = carouselWindow(frames, 1), after = carouselWindow(frames, 2);
  assert.deepEqual(before.map((slide) => [slide.key, slide.offset]), [["1", -1], ["7", 0], ["10", 1]]);
  assert.equal(before[2].key, after[1].key);
  assert.deepEqual(carouselWindow(frames, 3).map((slide) => slide.offset), [-1, 0]);
  assert.deepEqual(carouselWindow(frames, 0).map((slide) => slide.offset), [0, 1]);
  assert.match(previewMarkup({ offset: -1, index: 1, title: '<script>bad()</script>' }), /&lt;script&gt;/);
});

test("one stalled action occupies one carousel card throughout construction and retry", () => {
  const frames = [
    { event: { seq: 1, kind: "seed" } },
    ...[2, 4, 5].map((seq) => ({ event: { seq, kind: "outcome" }, workflow: { id: "action:1" } })),
    { event: { seq: 6, kind: "outcome" } },
  ];
  for (const selected of [1, 2, 3]) {
    assert.deepEqual(carouselWindow(frames, selected).map((f) => [f.key, f.offset]), [["1", -1], ["action:1", 0], ["6", 1]]);
  }
});
