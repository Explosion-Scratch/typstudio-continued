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
      });
      continue;
    }

    const listMatch = line.match(/^(\s*)[-*+]\s+/);
    if (listMatch && i === 0 || (listMatch && i > 0 && !lines[i - 1].match(/^(\s*)[-*+]\s+/))) {
      outline.push({
        type: "list",
        level: 2,
        title: "List",
        line: lineNumber,
      });
    }
  }

  return outline;
}
