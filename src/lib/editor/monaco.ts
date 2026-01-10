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
    { token: "keyword", foreground: "d44c47" },
    { token: "string", foreground: "448c27" },
    { token: "number", foreground: "9d5127" },
    { token: "operator", foreground: "37352f" },
    { token: "delimiter", foreground: "6b6b6b" },
    { token: "type", foreground: "6940a5" },
    { token: "function", foreground: "2383e2" },
    { token: "variable", foreground: "37352f" },
    { token: "constant", foreground: "9d5127" },
    { token: "markup.heading", foreground: "37352f", fontStyle: "bold" },
    { token: "markup.bold", fontStyle: "bold" },
    { token: "markup.italic", fontStyle: "italic" },
    { token: "markup.raw", foreground: "d44c47" },
    { token: "markup.link", foreground: "2383e2" },
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

