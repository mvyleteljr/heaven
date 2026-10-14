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
  assert.equal(root.children.find((node) => node.kind === "domain").children[0].children[0].children[0].fields.balance, 3000);
  assert.notEqual(before.id, root.children.find((node) => node.kind === "domain").children[0].children[0].children[0].id);
  assert.equal(root.count, 6);
  delete changed.ontology.modules.treasury;
  assert.equal(findDomain(worldTree(changed), root.children.find((node) => node.kind === "domain").id), null);
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
  const root = worldTree(world), record = root.children.find((node) => node.kind === "domain").children[0].children[0].children[0];
  const html = renderStructure(world, "domains", record.id);
  assert.match(html, /aria-current="location">Operating/);
  assert.match(html, /<dd>3000<\/dd>/);
  assert.match(html, /data-world-focus="world"/);
  assert.match(renderStructure({}, "domains", record.id), /aria-current="location">World/);
  assert.match(renderStructure(world, "tree"), /Clerk transfers money/);
  const unsafe = { ontology: { snapshot: { instances: [{ id: '<img onerror="bad()">', type: "Thing", fields: { name: '<script>bad()</script>' } }] } } };
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


test("ontology contains declared concepts and Story groupings, never transport resources", () => {
  const foundation = {
    definition: { source: { "model.onto": "project RepairCompany\nactor Operator\nobject Company\n" } },
    snapshot: { instances: [{ id: "Operator:Ada", type: "Operator", fields: { name: "Ada" } }] },
  };
  const state = { ontology: { ...foundation, modules: { "repair-intake": {
    definition: { source: { "model.onto": foundation.definition.source["model.onto"] + "object RepairRequest\n  status: string\nstory Intake\n  Operator creates RepairRequest\n    sets RepairRequest.status to \"pending\"\n  Operator reviews RepairRequest\n    sets RepairRequest.status to \"reviewed\"\n" } },
    snapshot: foundation.snapshot,
    protected_types: ["Operator", "Company"],
    stories: [{ id: 1, story: "Operator creates RepairRequest" }, { id: 2, story: "Operator reviews RepairRequest" }],
  } } }, tools: { public_resources: { "https://institutions.local/": { title: "API root" }, "https://other.local/": {} }, workspace: { files: { "debug.txt": "text" } }, messages: [{ subject: "Transport" }] }, hosting: { service: "https://service.local/" }, documents: { manual: {} } };
  const root = worldTree(state), [actors, objects, module] = root.children;
  assert.equal(root.name, "Institution");
  assert.equal(root.children.length, 3);
  assert.equal(actors.name, "Shared actors");
  assert.equal(objects.name, "Shared objects");
  assert.deepEqual([actors.children[0].kind, objects.children[0].kind], ["actor", "object"]);
  assert.equal(module.name, "Repair intake");
  assert.deepEqual(module.children.map((child) => child.name), ["Objects", "Stories"]);
  assert.equal(module.children[0].children[0].instanceCount, 0, "uninstantiated types remain visible with zero instances");
  assert.equal(module.children[1].children[0].storyCount, 2, "authored Story grouping is retained");
  assert.equal(module.children[1].children[0].children.length, 0, "Story names and sentences do not make redundant concentric circles");
  assert.deepEqual(module.references, [{name: "Operator", id: actors.children[0].id}]);
  for (const view of ["tree", "domains"]) {
    const html = renderStructure(state, view);
    assert.doesNotMatch(html, /institutions\.local|other\.local|Workspace|Mailbox|Hosted services|API root|debug\.txt|Repair Company|Open Institution/);
    assert.match(html, /RepairRequest/);
  }
  const changed = structuredClone(state);
  changed.ontology.modules["repair-intake"].snapshot = { instances: [{ id: "RepairRequest:1", type: "RepairRequest", fields: { status: "pending" } }] };
  const type = findDomain(worldTree(changed), module.children[0].children[0].id);
  assert.equal(type.children[0].fields.status, "pending");
  assert.equal(type.instanceCount, 1);
  const before = structuredClone(state); before.ontology.modules = {};
  const published = renderStructure(state, "domains", "world", before);
  assert.match(published, /Domain added: Repair intake/);
  assert.match(published, /0 instances/);
  assert.doesNotMatch(published, /Instance created:/);
  assert.match(published, /Uses shared/);
  assert.doesNotMatch(published, /is-empty|Dashed:/);
  const executed = renderStructure(changed, "domains", "world", state);
  assert.match(executed, /Instance created: RepairRequest:1 · status: pending/);
  assert.doesNotMatch(executed, /Domain added:/);
  const progressed = structuredClone(changed);
  progressed.ontology.modules["repair-intake"].snapshot.instances[0].fields.status = "resolved";
  assert.match(renderStructure(progressed, "domains", "world", changed), /pending → resolved/);
  assert.equal(findDomain(worldTree(changed), actors.children[0].id).children[0].name, "Ada");
  assert.match(renderStructure(state, "domains", module.id), /aria-current="location">Repair intake/);
  assert.match(renderStructure({ tools: state.tools }, "domains"), /No domain concepts/);
});


test("overview uses one solid concept circle and reveals instances only on drill-down", () => {
  const root = worldTree(world);
  const overview = packDomain(root);
  assert.ok(overview.descendants().every(({data}) => !["category", "instance"].includes(data.kind)));
  const type = root.children[0].children[0];
  assert.equal(overview.descendants().filter(({data}) => data.id === type.id).length, 1);
  assert.equal(overview.descendants().find(({data}) => data.id === type.id).children, undefined);
  const detail = packDomain(type);
  assert.equal(detail.children.length, 2);
  assert.deepEqual(detail.children.map(({data}) => data.name), ["Operating", "Reserve"]);
  assert.match(renderStructure(world, "domains", type.id), /Inspect Operating/);
  const svg = renderStructure(world, "domains").match(/<svg[\s\S]*?<\/svg>/)[0];
  assert.doesNotMatch(svg, /Inspect Operating|kind-category|is-empty/);
  const changed = structuredClone(world);
  changed.ontology.snapshot.instances.push({id:"Account:New",type:"Account",fields:{name:"New",balance:0}});
  const after = packDomain(worldTree(changed));
  assert.deepEqual(after.descendants().map(({data,x,y,r}) => [data.id,x,y,r]), overview.descendants().map(({data,x,y,r}) => [data.id,x,y,r]), "instance additions update counts without introducing rings in the overview");
});
