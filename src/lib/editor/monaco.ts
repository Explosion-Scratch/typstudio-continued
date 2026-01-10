import * as monaco from "monaco-editor";
import * as oniguruma from "vscode-oniguruma";
import onigurumaWasm from "vscode-oniguruma/release/onig.wasm?url";
import { Registry } from "vscode-textmate";

import { wireTextMateGrammars } from "./grammar";
import bibtex from "./lang/bibtex.json";
import typstConfig from "./lang/typst-config.json";
import typstTm from "./lang/typst-tm.json";

import { TypstCompletionProvider } from "$lib/editor/completion";

type IMonarchLanguage = monaco.languages.IMonarchLanguage;

const notionLightTheme: monaco.editor.IStandaloneThemeData = {
  base: "vs",
  inherit: true,
  rules: [
    { token: "comment", foreground: "9b9a97", fontStyle: "italic" },
    { token: "comment.line.double-slash.typst", foreground: "9b9a97", fontStyle: "italic" },
    { token: "comment.block.typst", foreground: "9b9a97", fontStyle: "italic" },
    { token: "keyword", foreground: "d44c47" },
    { token: "keyword.other.typst", foreground: "d44c47" },
    { token: "keyword.control", foreground: "d44c47" },
    { token: "keyword.control.conditional.typst", foreground: "d44c47" },
    { token: "keyword.control.loop.typst", foreground: "d44c47" },
    { token: "keyword.control.import.typst", foreground: "d44c47" },
    { token: "keyword.control.flow.typst", foreground: "d44c47" },
    { token: "keyword.operator", foreground: "37352f" },
    { token: "keyword.operator.typst", foreground: "37352f" },
    { token: "string", foreground: "448c27" },
    { token: "string.quoted.double.typst", foreground: "448c27" },
    { token: "number", foreground: "9d5127" },
    { token: "constant.numeric", foreground: "9d5127" },
    { token: "constant.numeric.integer.typst", foreground: "9d5127" },
    { token: "constant.numeric.float.typst", foreground: "9d5127" },
    { token: "constant.numeric.length.typst", foreground: "9d5127" },
    { token: "constant.numeric.angle.typst", foreground: "9d5127" },
    { token: "constant.numeric.percentage.typst", foreground: "9d5127" },
    { token: "constant.language", foreground: "9d5127" },
    { token: "constant.language.none.typst", foreground: "9d5127" },
    { token: "constant.language.auto.typst", foreground: "9d5127" },
    { token: "constant.language.boolean.typst", foreground: "9d5127" },
    { token: "operator", foreground: "37352f" },
    { token: "delimiter", foreground: "6b6b6b" },
    { token: "type", foreground: "6940a5" },
    { token: "function", foreground: "2383e2" },
    { token: "entity.name.function", foreground: "2383e2" },
    { token: "entity.name.function.typst", foreground: "2383e2" },
    { token: "variable", foreground: "37352f" },
    { token: "variable.other.typst", foreground: "37352f" },
    { token: "variable.parameter.typst", foreground: "e67e22", fontStyle: "italic" },
    { token: "entity.other.interpolated.typst", foreground: "6940a5" },
    { token: "constant", foreground: "9d5127" },
    { token: "markup.heading", foreground: "c026d3", fontStyle: "bold" },
    { token: "markup.heading.typst", foreground: "c026d3", fontStyle: "bold" },
    { token: "entity.name.section.typst", foreground: "c026d3", fontStyle: "bold" },
    { token: "punctuation.definition.heading.typst", foreground: "c026d3", fontStyle: "bold" },
    { token: "markup.bold", foreground: "37352f", fontStyle: "bold" },
    { token: "markup.bold.typst", foreground: "37352f", fontStyle: "bold" },
    { token: "punctuation.definition.bold.typst", foreground: "37352f", fontStyle: "bold" },
    { token: "markup.italic", foreground: "37352f", fontStyle: "italic" },
    { token: "markup.italic.typst", foreground: "37352f", fontStyle: "italic" },
    { token: "punctuation.definition.italic.typst", foreground: "37352f", fontStyle: "italic" },
    { token: "markup.raw", foreground: "d44c47" },
    { token: "markup.raw.inline.typst", foreground: "d44c47" },
    { token: "markup.raw.block.typst", foreground: "d44c47" },
    { token: "markup.link", foreground: "2383e2" },
    { token: "markup.underline.link.typst", foreground: "2383e2" },
    { token: "meta.math.typst", foreground: "8b5cf6" },
    { token: "entity.other.label.typst", foreground: "e67e22" },
    { token: "entity.other.reference.typst", foreground: "e67e22" },
    { token: "punctuation.definition.list", foreground: "6b6b6b" },
    { token: "punctuation.definition.list.unnumbered.typst", foreground: "6b6b6b" },
    { token: "punctuation.definition.list.numbered.typst", foreground: "6b6b6b" },
    { token: "constant.symbol.typst", foreground: "8b5cf6" },
  ],
  colors: {
    "editor.background": "#ffffff",
    "editor.foreground": "#37352f",
    "editor.lineHighlightBackground": "#f7f6f380",
    "editor.selectionBackground": "#2383e233",
    "editor.inactiveSelectionBackground": "#2383e21a",
    "editorCursor.foreground": "#37352f",
    "editorLineNumber.foreground": "#c4c4c4",
    "editorLineNumber.activeForeground": "#6b6b6b",
    "editorIndentGuide.background": "#e8e8e8",
    "editorIndentGuide.activeBackground": "#d4d4d4",
    "editorBracketMatch.background": "#2383e233",
    "editorBracketMatch.border": "#2383e280",
    "editorGutter.background": "#ffffff",
    "editorWidget.background": "#ffffff",
    "editorWidget.border": "#e0dfdc",
    "editorSuggestWidget.background": "#ffffff",
    "editorSuggestWidget.border": "#e0dfdc",
    "editorSuggestWidget.selectedBackground": "#f7f6f3",
    "editorSuggestWidget.highlightForeground": "#2383e2",
    "list.hoverBackground": "#f7f6f3",
    "scrollbar.shadow": "#00000008",
    "scrollbarSlider.background": "#37352f1a",
    "scrollbarSlider.hoverBackground": "#37352f2a",
    "scrollbarSlider.activeBackground": "#37352f3a",
  },
};

export const initMonaco = (async () => {
  const wasm = await fetch(onigurumaWasm).then((res) => res.arrayBuffer());
  await oniguruma.loadWASM(wasm);

  const registry = new Registry({
    onigLib: Promise.resolve(oniguruma),
    loadGrammar() {
      return Promise.resolve(typstTm);
    },
  });

  const grammars = new Map();
  grammars.set("typst", "source.typst");

  monaco.languages.register({ id: "typst", extensions: ["typ"] });
  monaco.languages.setLanguageConfiguration(
    "typst",
    typstConfig as unknown as monaco.languages.LanguageConfiguration
  );
  await wireTextMateGrammars(registry, { typst: "source.typst" });

  monaco.languages.register({ id: "bibtex", extensions: ["bib"] });
  monaco.languages.setMonarchTokensProvider("bibtex", bibtex as IMonarchLanguage);

  monaco.languages.registerCompletionItemProvider("typst", new TypstCompletionProvider());

  monaco.editor.defineTheme("notion-light", notionLightTheme);
  monaco.editor.setTheme("notion-light");
})();

