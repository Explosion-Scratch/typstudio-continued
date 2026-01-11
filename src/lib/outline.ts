import type { OutlineItem } from "./stores";

export function extractOutlineFromSource(content: string): OutlineItem[] {
  const outline: OutlineItem[] = [];
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNumber = i + 1;

    const headingMatch = line.match(/^(=+)\s+(.+?)(?:\s*<[^>]+>)?$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const title = headingMatch[2].trim();
      outline.push({
        type: "heading",
        level,
        title,
        line: lineNumber,
        endLine: lineNumber, // Will be updated
      });
      continue;
    }

    const figureMatch = line.match(/#figure\s*\(/);
    if (figureMatch) {
      let captionMatch = line.match(/caption:\s*\[([^\]]*)\]/);
      if (!captionMatch) {
        for (let j = i + 1; j < Math.min(i + 10, lines.length); j++) {
          captionMatch = lines[j].match(/caption:\s*\[([^\]]*)\]/);
          if (captionMatch) break;
        }
      }
      const title = captionMatch ? captionMatch[1].trim() : "Figure";
      outline.push({
        type: "figure",
        level: 2,
        title: `Fig: ${title}`,
        line: lineNumber,
        endLine: lineNumber, // Usually single line or small block, for now same
      });
      continue;
    }

    const tableMatch = line.match(/#table\s*\(/);
    if (tableMatch) {
      outline.push({
        type: "table",
        level: 2,
        title: "Table",
        line: lineNumber,
        endLine: lineNumber,
      });
      continue;
    }

    const includeMatch = line.match(/#include\s+"([^"]+)"/);
    if (includeMatch) {
      outline.push({
        type: "include",
        level: 2,
        title: `Include: ${includeMatch[1]}`,
        line: lineNumber,
        endLine: lineNumber,
      });
      continue;
    }

    const listMatch = line.match(/^(\s*)[-*+]\s+/);
    if (listMatch && i === 0 || (listMatch && i > 0 && !lines[i - 1].match(/^(\s*)[-*+]\s+/))) {
      // Count list items
      let count = 0;
      let lastLine = i;
      for (let j = i; j < lines.length; j++) {
        if (lines[j].match(/^(\s*)[-*+]\s+/)) {
          count++;
          lastLine = j;
        } else if (lines[j].trim() === "") {
          continue;
        } else {
          break;
        }
      }

      outline.push({
        type: "list",
        level: 2,
        title: `List (${count})`,
        line: lineNumber,
        endLine: lastLine + 1,
      });
    }
  }

  // Post-process headings to set endLine based on next heading or end of file
  for (let i = 0; i < outline.length; i++) {
    const item = outline[i];
    if (item.type === "heading") {
      let nextHeadingIdx = -1;
      for (let j = i + 1; j < outline.length; j++) {
        if (outline[j].type === "heading" && outline[j].level <= item.level) {
          nextHeadingIdx = j;
          break;
        }
      }

      if (nextHeadingIdx !== -1) {
        item.endLine = outline[nextHeadingIdx].line - 1;
      } else {
        item.endLine = lines.length;
      }
    }
  }

  return outline;
}
