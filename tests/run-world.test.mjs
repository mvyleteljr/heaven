import test from "node:test";
import assert from "node:assert/strict";
import { transitionState } from "../web/run-world.js";

function scene() {
  const nodes = [], calls = [];
  function element(height, parentElement = null) {
    const node = { nodeType: 1, namespaceURI: "http://www.w3.org/1999/xhtml", height, opacity: "1", parentElement,
      getBoundingClientRect() { return { height: this.renderedHeight ?? this.height }; },
      animate(frames, options) {
        let finish;
        const motion = { frames, options, canceled: false, finished: new Promise((resolve) => { finish = resolve; }),
          cancel() { this.canceled = true; delete node.renderedHeight; node.opacity = "1"; }, finish: () => finish() };
        calls.push({ node, motion }); return motion;
      },
    };
    return node;
  }
  const root = element(100);
  root.querySelectorAll = () => nodes;
  root.contains = (node) => node === root || nodes.includes(node);
  const env = { reduced: false, matchMedia: () => ({ matches: env.reduced }), getComputedStyle: (node) => ({ opacity: node.opacity, display: "block", paddingTop: "10px", paddingBottom: "10px", marginTop: "0px", marginBottom: "8px" }) };
  return { root, env, nodes, calls, element };
}

test("unchanged state does not animate; changed values fade without panel translation", () => {
  const { root, env, nodes, calls, element } = scene();
  const value = element(20, root); nodes.push(value);
  transitionState(root, () => {}, env);
  assert.equal(calls.length, 0);
  transitionState(root, () => ({ changed: new Set([value]) }), env);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].node, value);
  assert.deepEqual(calls[0].motion.frames, [{ opacity: 0.35 }, { opacity: 1 }]);
});

test("new content grows in layout and fades while its container adjusts height", () => {
  const { root, env, nodes, calls, element } = scene();
  const section = element(80, root);
  transitionState(root, () => { root.height = 180; nodes.push(section); return { added: new Set([section]) }; }, env);
  const container = calls.find((call) => call.node === root).motion.frames;
  assert.equal(container[0].height, "100px"); assert.equal(container[1].height, "180px");
  const entering = calls.find((call) => call.node === section).motion.frames;
  assert.equal(entering[0].height, "0px"); assert.equal(entering[1].height, "80px");
  assert.equal(entering[0].opacity, 0); assert.equal(entering[1].opacity, 1);
  assert.equal(entering[0].paddingTop, "0px");
  assert.ok(calls.every(({ motion }) => motion.frames.every((frame) => !("transform" in frame))));
});

test("rapid updates retarget from current geometry and do not leave fixed heights", async () => {
  const { root, env, calls } = scene();
  transitionState(root, () => { root.height = 200; }, env);
  const first = calls[0].motion;
  root.renderedHeight = 140;
  transitionState(root, () => { root.height = 80; }, env);
  const second = calls[1].motion;
  assert.ok(first.canceled);
  assert.equal(second.frames[0].height, "140px"); assert.equal(second.frames[1].height, "80px");
  assert.equal(second.options.fill, undefined);
  first.finish(); await Promise.resolve();
  root.renderedHeight = 110;
  transitionState(root, () => { root.height = 150; }, env);
  assert.ok(second.canceled, "an older completion must not discard the current transition");
  assert.equal(calls[2].motion.frames[0].height, "110px");
});

test("reduced motion cancels pending transitions and applies new state immediately", () => {
  const { root, env, calls } = scene();
  transitionState(root, () => { root.height = 200; }, env);
  env.reduced = true;
  transitionState(root, () => { root.height = 50; }, env);
  assert.ok(calls[0].motion.canceled);
  assert.equal(calls.length, 1);
  assert.equal(root.height, 50);
});
