const transitions = new WeakMap();
const layoutSelector = [
  ".stage-card", ".agent-state-card", ".agent-budget", ".agent-state-metrics", ".agent-state-metrics > div",
  ".snapshot", ".fact-record", ".fact-row", ".fact-value", "details", "pre",
  "svg circle", ".domain-list", ".domain-fields", ".domain-fields > div", ".domain-map", ".pack-children", ".domain-inspector", ".ontology-changes", ".concept-references",
].join(",");
const duration = 360;
const easing = "cubic-bezier(.22,1,.36,1)";

// Measure before and after one synchronous mutation. Animate actual layout
// height, never translate the panel or scale its text. Retarget interrupted work
// from its rendered size; no fixed heights remain when the animation finishes.
export function transitionState(root, mutate, env = root.ownerDocument.defaultView) {
  let state = transitions.get(root);
  if (!state) { state = { motions: new Map() }; transitions.set(root, state); }
  const reduced = env.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  const measure = (element) => {
    const style = env.getComputedStyle(element);
    return { height: element.getBoundingClientRect().height, opacity: style.opacity,
      circle: element.tagName?.toLowerCase() === "circle" ? Object.fromEntries(["cx", "cy", "r"].map((key) => {
        const rendered = Number.parseFloat(style[key]);
        return [key, Number.isFinite(rendered) ? rendered : Number(element.getAttribute(key))];
      })) : null };
  };
  const prior = new Map();
  if (!reduced) {
    for (const element of new Set([root, ...root.querySelectorAll(layoutSelector), ...state.motions.keys()]))
      if (element === root || root.contains(element)) prior.set(element, measure(element));
  }
  const interrupted = new Set(state.motions.keys());
  for (const motion of state.motions.values()) motion.cancel();
  state.motions.clear();
  const changes = mutate() || {};
  if (reduced) return;

  const fades = new Set([...(changes.added || []), ...(changes.changed || [])].filter((node) => node?.nodeType === 1 && root.contains(node)));
  // A new section fades as one surface; don't multiply its children's opacity.
  for (const node of [...fades]) {
    for (let parent = node.parentElement; parent && parent !== root; parent = parent.parentElement)
      if (fades.has(parent)) { fades.delete(node); break; }
  }
  const targets = new Set([root, ...root.querySelectorAll(layoutSelector), ...fades]);
  const plans = [];
  // Complete all layout reads before starting any animations.
  for (const node of targets) {
    if (!node.animate) continue;
    // One disclosure may reveal hundreds of rows. Animate its enclosing box
    // once instead of multiplying nested fades and layout animations.
    if (changes.disclosure && node !== changes.disclosure && changes.disclosure.contains(node)) continue;
    const after = measure(node), before = prior.get(node);
    if (after.height === 0 && !before?.height) continue;
    const start = {}, end = {};
    if (after.circle) {
      for (const property of ["cx", "cy", "r"]) {
        if (before?.circle && before.circle[property] !== after.circle[property]) {
          start[property] = `${before.circle[property]}px`; end[property] = `${after.circle[property]}px`;
        } else if (!before && property === "r") {
          start.r = "0px"; end.r = `${after.circle.r}px`;
          start.opacity = 0; end.opacity = 1;
        }
      }
    }
    const html = node.namespaceURI === "http://www.w3.org/1999/xhtml";
    const style = env.getComputedStyle(node);
    const newBlock = changes.added?.has(node) && html && ["block", "flex", "grid", "list-item", "flow-root"].includes(style.display);
    if (html && prior.has(node) && Math.abs(before.height - after.height) > 0.5) {
      start.height = `${before.height}px`; end.height = `${after.height}px`;
      start.overflow = end.overflow = "clip";
    }
    if (newBlock) {
      start.height = "0px"; end.height = `${after.height}px`;
      start.overflow = end.overflow = "clip";
      for (const property of ["paddingTop", "paddingBottom", "marginTop", "marginBottom"]) {
        start[property] = "0px"; end[property] = style[property];
      }
    }
    if (fades.has(node) || (before?.height === 0 && after.height > 0)) {
      start.opacity = before ? interrupted.has(node) ? before.opacity : before.height === 0 ? 0 : 0.35 : 0;
      end.opacity = 1;
    } else if (before && Number(before.opacity) < 1) {
      start.opacity = before.opacity; end.opacity = 1;
    }
    if (Object.keys(start).length) plans.push([node, start, end]);
  }
  for (const [node, start, end] of plans) {
    const motion = node.animate([start, end], { duration, easing });
    state.motions.set(node, motion);
    const cleanup = () => { if (state.motions.get(node) === motion) state.motions.delete(node); };
    motion.finished.then(cleanup, cleanup);
  }
}

function enableDisclosureMotion(root) {
  if (root.dataset.stateDisclosures === "true") return;
  root.dataset.stateDisclosures = "true";
  root.addEventListener("click", (event) => {
    const summary = event.target.closest?.("summary");
    if (!summary || !root.contains(summary) || summary.parentElement?.tagName !== "DETAILS" || event.defaultPrevented) return;
    event.preventDefault();
    const details = summary.parentElement;
    transitionState(root, () => { details.open = !details.open; return { disclosure: details, changed: new Set([details]) }; });
  });
}

// Retain cards, open disclosures and focus. Keys describe record identity, not
// the changing value. A no-op refresh produces no animation or DOM replacement.
export function reconcileWorld(root, html) {
  enableDisclosureMotion(root);
  const template = root.ownerDocument.createElement("template");
  template.innerHTML = html;
  const key = (node) => node.nodeType === 1 ? node.getAttribute("data-world-key") : null;
  const compatible = (a, b) => a.nodeType === b.nodeType && a.nodeName === b.nodeName && key(a) === key(b);
  const added = new Set(), changed = new Set();
  function patch(target, source) {
    if (target.nodeType !== 1) {
      if (target.nodeValue !== source.nodeValue) {
        target.nodeValue = source.nodeValue;
        changed.add(target.parentElement);
      }
      return;
    }
    for (const attribute of [...target.attributes]) {
      if (target.nodeName === "DETAILS" && attribute.name === "open") continue;
      if (!source.hasAttribute(attribute.name)) target.removeAttribute(attribute.name);
    }
    for (const attribute of source.attributes) {
      if (target.nodeName === "DETAILS" && attribute.name === "open") continue;
      if (target.getAttribute(attribute.name) !== attribute.value) {
        target.setAttribute(attribute.name, attribute.value);
        if (["value", "cx", "cy", "r"].includes(attribute.name)) changed.add(target);
      }
    }
    children(target, source);
  }
  function children(target, source) {
    const existing = [...target.childNodes], used = new Set();
    let cursor = target.firstChild;
    for (const desired of [...source.childNodes]) {
      const retained = existing.find((node) => !used.has(node) && compatible(node, desired));
      const node = retained || desired.cloneNode(true);
      if (retained) { used.add(node); patch(node, desired); }
      else if (node.nodeType === 1) added.add(node);
      else changed.add(target);
      if (node !== cursor) target.insertBefore(node, cursor);
      cursor = node.nextSibling;
    }
    for (const node of existing) if (!used.has(node)) node.remove();
  }
  transitionState(root, () => {
    children(root, template.content);
    return { added, changed };
  });
}
