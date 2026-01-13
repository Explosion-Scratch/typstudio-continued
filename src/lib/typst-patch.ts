const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

export function patchSvgToContainer(
  container: Element,
  svgString: string,
  decorateSvgElement?: (elem: SVGElement) => void
): void {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svgString, "image/svg+xml");
  const nextSvg = doc.documentElement as unknown as SVGElement;

  if (nextSvg.tagName !== "svg") {
    container.innerHTML = svgString;
    return;
  }

  const prevSvg = container.querySelector("svg");

  if (!prevSvg) {
    container.innerHTML = svgString;
    const inserted = container.querySelector("svg");
    if (inserted && decorateSvgElement) {
      decorateSvgElement(inserted);
    }
    return;
  }

  const prevTid = prevSvg.getAttribute("data-tid");
  const nextTid = nextSvg.getAttribute("data-tid");

  if (prevTid && nextTid && prevTid === nextTid) {
    return;
  }

  patchSvgElement(prevSvg, nextSvg);

  if (decorateSvgElement) {
    decorateSvgElement(prevSvg);
  }
}

function patchSvgElement(prev: SVGElement, next: SVGElement): void {
  patchAttributes(prev, next);
  patchChildren(prev, next);
}

function patchAttributes(prev: Element, next: Element): void {
  const prevAttrs = prev.attributes;
  const nextAttrs = next.attributes;

  const toRemove: string[] = [];
  for (let i = 0; i < prevAttrs.length; i++) {
    const name = prevAttrs[i].name;
    if (!next.hasAttribute(name)) {
      toRemove.push(name);
    }
  }
  toRemove.forEach((name) => prev.removeAttribute(name));

  for (let i = 0; i < nextAttrs.length; i++) {
    const attr = nextAttrs[i];
    if (prev.getAttribute(attr.name) !== attr.value) {
      prev.setAttribute(attr.name, attr.value);
    }
  }
}

function patchChildren(prev: Element, next: Element): void {
  const prevChildren = Array.from(prev.children);
  const nextChildren = Array.from(next.children);

  const prevByTid = new Map<string, Element>();
  prevChildren.forEach((child) => {
    const tid = child.getAttribute("data-tid");
    if (tid) {
      prevByTid.set(tid, child);
    }
  });

  const usedPrevIndices = new Set<number>();
  const operations: Array<{type: "reuse" | "insert", prevIdx?: number, nextChild: Element}> = [];

  nextChildren.forEach((nextChild) => {
    const tid = nextChild.getAttribute("data-tid");

    if (tid && prevByTid.has(tid)) {
      const prevChild = prevByTid.get(tid)!;
      const prevIdx = prevChildren.indexOf(prevChild);

      if (!usedPrevIndices.has(prevIdx)) {
        usedPrevIndices.add(prevIdx);
        operations.push({ type: "reuse", prevIdx, nextChild });
      } else {
        operations.push({ type: "insert", nextChild });
      }
    } else {
      operations.push({ type: "insert", nextChild });
    }
  });

  prevChildren.forEach((child, idx) => {
    if (!usedPrevIndices.has(idx)) {
      child.remove();
    }
  });

  let currentChildren = Array.from(prev.children);

  operations.forEach((op, targetIdx) => {
    if (op.type === "reuse" && op.prevIdx !== undefined) {
      const prevChild = currentChildren.find(
        (c) => c.getAttribute("data-tid") === op.nextChild.getAttribute("data-tid")
      );

      if (prevChild) {
        const currentIdx = Array.from(prev.children).indexOf(prevChild);

        if (currentIdx !== targetIdx) {
          const refNode = prev.children[targetIdx];
          if (refNode && refNode !== prevChild) {
            prev.insertBefore(prevChild, refNode);
          }
        }

        if (prevChild.tagName === "g") {
          patchSvgElement(prevChild as SVGElement, op.nextChild as SVGElement);
        } else {
          patchAttributes(prevChild, op.nextChild);
        }
      }
    } else {
      const refNode = prev.children[targetIdx];
      if (refNode) {
        prev.insertBefore(op.nextChild, refNode);
      } else {
        prev.appendChild(op.nextChild);
      }
    }

    currentChildren = Array.from(prev.children);
  });
}

export function shouldPatch(prev: SVGElement | null, nextString: string): boolean {
  if (!prev) return false;

  const tidMatch = nextString.match(/data-tid="([^"]+)"/);
  if (!tidMatch) return false;

  const nextTid = tidMatch[1];
  const prevTid = prev.getAttribute("data-tid");

  return prevTid !== nextTid;
}
