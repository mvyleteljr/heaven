import { reconcileWorld } from "./run-world.js";
import { agentPreview, actionState, esc, renderAgent } from "./run-view.js";

export function carouselWindow(frames, selected) {
  if (!frames[selected]) return [];
  const key = (index) => agentPreview(frames[index], index).key;
  const current = key(selected);
  let previous = selected - 1, next = selected + 1;
  while (previous >= 0 && key(previous) === current) --previous;
  while (next < frames.length && key(next) === current) ++next;
  return [[previous, -1], [selected, 0], [next, 1]].filter(([index]) => frames[index]).map(([index, offset]) => ({
    ...agentPreview(frames[index], index), offset, frame: frames[index],
  }));
}

export function previewMarkup(slide) {
  const direction = slide.offset < 0 ? "Previous" : "Next";
  return `<span class="preview-label">${direction} · checkpoint ${slide.index + 1}<span aria-hidden="true">${slide.offset < 0 ? "↑" : "↓"}</span></span><span class="preview-title serif">${esc(slide.title)}</span>`;
}

// Keep the same DOM card as it moves between the three positions. If navigation
// interrupts a transition, start the new one at its current on-screen geometry.
export function createAgentCarousel(root, env = globalThis) {
  const cards = new Map();
  let selection = "", selected = 0, lastUpdate, revision = 0, signature = "";
  const reduced = () => !!env.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  function geometry(offset) {
    const height = root.clientHeight;
    const density = (parseFloat(env.getComputedStyle(root).fontSize) || 16) / 16;
    const preview = (height < 320 * density ? 40 : height < 440 * density ? 62 : 76) * density;
    const gap = (height < 440 * density ? 10 : 14) * density;
    const main = Math.max(80 * density, height - 2 * (preview + gap));
    const scale = offset === 0 ? 1 : 0.9;
    const y = offset < 0 ? (offset + 1) * (preview + gap)
      : offset === 0 ? preview + gap : preview + gap + main + gap + (offset - 1) * (preview + gap);
    return { transform: `translateY(${y}px) scale(${scale})`, height: `${(offset === 0 ? main : preview) / scale}px`, opacity: Math.abs(offset) > 1 ? 0 : 1 };
  }
  function remove(card) {
    card.motion?.cancel();
    card.element.remove();
    cards.delete(card.key);
  }
  function update(frames, index, data, nextSelection, animate = true) {
    const changedSelection = selection !== nextSelection;
    if (changedSelection) {
      for (const card of cards.values()) remove(card);
      root.replaceChildren();
      selection = nextSelection;
    }
    const priorIndex = selected;
    selected = index;
    lastUpdate = [frames, index, data, nextSelection];
    const slides = carouselWindow(frames, index);
    const keys = new Set(slides.map((slide) => slide.key));
    const nextSignature = JSON.stringify([nextSelection, index, frames.length, slides.map((slide) => [slide.key, slide.frame.event.seq])]);
    if (!changedSelection && signature === nextSignature) return;
    signature = nextSignature;
    const token = ++revision;
    const sameAction = frames[priorIndex] && frames[index] && agentPreview(frames[priorIndex], priorIndex).key === agentPreview(frames[index], index).key;
    const moving = animate && !changedSelection && priorIndex !== index && !sameAction && !reduced();
    const direction = index >= priorIndex ? 1 : -1;
    const rootBox = root.getBoundingClientRect();
    const before = new Map();
    for (const card of cards.values()) {
      if (card.leaving && !keys.has(card.key)) { remove(card); continue; }
      const box = card.element.getBoundingClientRect();
      const scale = rootBox.width ? box.width / rootBox.width : 1;
      before.set(card.key, {
        transform: `translateY(${box.top - rootBox.top}px) scale(${scale})`,
        height: `${box.height / scale}px`, opacity: env.getComputedStyle(card.element).opacity,
      });
      card.motion?.cancel();
    }
    for (const slide of slides) {
      let card = cards.get(slide.key);
      if (!card) {
        const element = env.document.createElement("div");
        element.className = "agent-slide";
        element.dataset.slideKey = slide.key;
        element.setAttribute("role", "group");
        element.setAttribute("aria-roledescription", "slide");
        element.innerHTML = `<div class="agent-full" tabindex="0">${renderAgent(slide.frame, slide.index, data)}</div><button class="agent-preview" type="button"></button>`;
        card = { key: slide.key, element, body: element.querySelector(".agent-full"), preview: element.querySelector(".agent-preview") };
        cards.set(slide.key, card);
        root.append(element);
      }
      if (card.eventSeq !== slide.frame.event.seq) {
        if (card.eventSeq !== undefined) reconcileWorld(card.body, renderAgent(slide.frame, slide.index, data));
        card.eventSeq = slide.frame.event.seq;
      }
      const status = slide.frame.workflow?.phase ?? actionState(slide.frame);
      const buzz = slide.offset === 0 && ["blocked", "failed"].includes(status) && card.status !== status;
      card.status = status;
      card.element.dataset.state = status;
      if (buzz && animate && !reduced()) {
        card.body.animate?.([{ transform: "translateX(0)" }, { transform: "translateX(-4px)" }, { transform: "translateX(4px)" }, { transform: "translateX(-3px)" }, { transform: "translateX(2px)" }, { transform: "translateX(0)" }], { duration: 320, easing: "ease-out" });
      }
      card.index = slide.index;
      card.leaving = false;
      const current = slide.offset === 0;
      card.element.dataset.position = current ? "current" : slide.offset < 0 ? "previous" : "next";
      card.element.style.zIndex = current ? "3" : "2";
      card.element.removeAttribute("aria-hidden");
      card.element.inert = false;
      card.element.setAttribute("aria-label", `Checkpoint ${slide.index + 1} of ${frames.length}`);
      card.body.inert = !current;
      card.body.setAttribute("aria-hidden", String(!current));
      const focusedPreview = env.document.activeElement === card.preview;
      card.preview.hidden = current;
      card.preview.dataset.checkpoint = slide.index;
      card.preview.setAttribute("aria-label", `View ${slide.offset < 0 ? "previous" : "next"} checkpoint ${slide.index + 1}: ${slide.title}`);
      card.preview.innerHTML = previewMarkup(slide);
      if (current && focusedPreview) card.body.focus({ preventScroll: true });
      const end = geometry(slide.offset);
      Object.assign(card.element.style, end);
      if (moving && card.element.animate) {
        const start = before.get(slide.key) || geometry(slide.offset + direction);
        card.motion = card.element.animate([start, end], { duration: 520, easing: "cubic-bezier(.22,1,.36,1)" });
      }
    }
    for (const card of cards.values()) {
      if (keys.has(card.key)) continue;
      card.leaving = true;
      card.element.setAttribute("aria-hidden", "true");
      card.element.inert = true;
      const end = geometry(card.index < index ? -2 : 2);
      Object.assign(card.element.style, end);
      if (moving && card.element.animate) {
        card.motion = card.element.animate([before.get(card.key) || end, end], { duration: 520, easing: "cubic-bezier(.22,1,.36,1)" });
        card.motion.finished.then(() => { if (revision === token && card.leaving) remove(card); }).catch(() => {});
      } else remove(card);
    }
  }
  // Resizing is layout, not a new checkpoint. Do not replay the animation.
  const observer = env.ResizeObserver ? new env.ResizeObserver(() => {
    if (!lastUpdate) return;
    signature = "";
    update(...lastUpdate, false);
  }) : null;
  observer?.observe(root);
  return { update, destroy() { observer?.disconnect(); for (const card of cards.values()) remove(card); } };
}
