<script lang="ts">
  import { onMount } from "svelte";
  import type { editor } from "monaco-editor";
  import debounce from "lodash/debounce";

  import { initMonaco } from "../lib/editor/monaco";
  import type { TypstCompileEvent } from "../lib/ipc";
  import { compile, readFileText, writeFileText, jumpFromCursor } from "../lib/ipc";
  import { appWindow } from "@tauri-apps/api/window";
  import { paste } from "$lib/ipc/clipboard";
  import { PreviewState, shell } from "$lib/stores";
  import { extractOutlineFromSource } from "$lib/outline";

  type ICodeEditor = editor.ICodeEditor;
  type IModelContentChangedEvent = editor.IModelContentChangedEvent;
  type IModelChangedEvent = editor.IModelChangedEvent;
  type IMarkerData = editor.IMarkerData;

  let divEl: HTMLDivElement;
  let editorInstance: ICodeEditor;
  const monacoImport = import("monaco-editor");

  export let path: string;

  let isTyping = false;
  let lastCompileRequestId = 0;

  const updateOutline = () => {
    const model = editorInstance?.getModel();
    if (model) {
      const content = model.getValue();
      const outline = extractOutlineFromSource(content);
      shell.setDocumentOutline(outline);
    }
  };

  const updateOutlineDebounced = debounce(updateOutline, 300);

  const clearMarkersWhileTyping = () => {
    if (!isTyping) {
      isTyping = true;
      const model = editorInstance?.getModel();
      if (model) {
        import("monaco-editor").then((m) => {
          m.editor.setModelMarkers(model, "owner", []);
        });
      }
    }
  };

  const markTypingDone = debounce(() => {
    isTyping = false;
  }, 300);

  const handleCompile = async () => {
    const model = editorInstance?.getModel();
    if (model) {
      const filePath = model.uri.path;
      if (!filePath.endsWith(".typ")) {
        return;
      }
      const requestId = shell.nextCompileRequestId();
      lastCompileRequestId = requestId;
      shell.setPreviewState(PreviewState.Compiling);
      await compile(model.uri.path, model.getValue(), requestId);
    }
  };

  const handleSave = () => {
    const model = editorInstance.getModel();
    if (model) {
      writeFileText(model.uri.path, model.getValue());
    }
  };

  const handleSaveDebounce = debounce(handleSave, 1000, { maxWait: 5000 });

  const handleCursorJump = debounce(async () => {
    if (editorInstance) {
      const model = editorInstance.getModel();
      const position = editorInstance.getPosition();
      if (model && position && model.uri.path.endsWith(".typ")) {
        const offset = model.getOffsetAt(position);
        try {
          const result = await jumpFromCursor(model.uri.path, model.getValue(), offset);
          if (result) {
            appWindow.emit("scroll_to_position_in_preview", result);
          }
        } catch (e) {
          console.error("Failed to jump from cursor:", e);
        }
      }
    }
  }, 300);

  export const scrollToPosition = (line: number, column: number = 1) => {
    if (editorInstance) {
      editorInstance.revealLineInCenter(line);
      editorInstance.setPosition({ lineNumber: line, column });
      editorInstance.focus();
    }
  };

  export const getCursorPosition = () => {
    if (editorInstance) {
      const position = editorInstance.getPosition();
      const model = editorInstance.getModel();
      if (position && model) {
        return {
          line: position.lineNumber,
          column: position.column,
          offset: model.getOffsetAt(position),
        };
      }
    }
    return null;
  };

  onMount(() => {
    let cleanup: (() => void)[] = [];

    (async () => {
      const EditorWorker = await import("monaco-editor/esm/vs/editor/editor.worker?worker");
      await initMonaco;

      (self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
        getWorker: function (_moduleId: unknown, label: string) {
          return new EditorWorker.default();
        },
      };

      editorInstance = (await monacoImport).editor.create(divEl, {
        lineHeight: 1.8,
        automaticLayout: true,
        readOnly: true,
        folding: true,
        quickSuggestions: false,
        wordWrap: "on",
        unicodeHighlight: { ambiguousCharacters: false },
        padding: { top: 16 },
        minimap: { enabled: false },
        fontFamily: "var(--font-mono)",
        fontSize: 13,
        renderLineHighlight: "gutter",
        scrollbar: {
          vertical: "auto",
          horizontal: "auto",
          verticalScrollbarSize: 8,
          horizontalScrollbarSize: 8,
        },
      });

      editorInstance.onDidChangeModel((e: IModelChangedEvent) => {
        handleCompile();
        updateOutline();
      });

      editorInstance.onDidChangeModelContent((e: IModelContentChangedEvent) => {
        clearMarkersWhileTyping();
        markTypingDone();
        handleCompile();
        handleSaveDebounce();
        updateOutlineDebounced();
      });

      editorInstance.onDidChangeCursorPosition(() => {
        const pos = getCursorPosition();
        if (pos) {
          appWindow.emit("editor_cursor_changed", pos);
          handleCursorJump();
        }
      });

      const unsubscribeCompile = await appWindow.listen<TypstCompileEvent>("typst_compile", ({ payload }) => {
        const { document, diagnostics } = payload;
        const model = editorInstance.getModel();
        if (model && diagnostics && !isTyping) {
          import("monaco-editor").then((m) => {
            const markers: IMarkerData[] = diagnostics.map(({ range, severity, message, hints }) => {
              const start = model.getPositionAt(range.start);
              const end = model.getPositionAt(range.end);
              return {
                startLineNumber: start.lineNumber,
                startColumn: start.column,
                endLineNumber: end.lineNumber,
                endColumn: end.column,
                message: message + "\n" + hints.map((hint) => `hint: ${hint}`).join("\n"),
                severity: severity === "error" ? m.MarkerSeverity.Error : m.MarkerSeverity.Warning,
              };
            });
            m.editor.setModelMarkers(model, "owner", markers);
          });
        }
        if (document) {
          shell.setPreviewState(PreviewState.Idle);
        } else {
          shell.setPreviewState(PreviewState.CompileError);
        }
      });
      cleanup.push(unsubscribeCompile);

      const unsubscribeJumpTo = await appWindow.listen<{ line: number; column?: number }>("jump_to_position", ({ payload }) => {
        scrollToPosition(payload.line, payload.column || 1);
      });
      cleanup.push(unsubscribeJumpTo);
    })();

    return () => {
      cleanup.forEach((fn) => fn());
      if (editorInstance) editorInstance.dispose();
    };
  });

  const fetchContent = async (editor: ICodeEditor, editorPath: string) => {
    if (!editor) return;

    editor.updateOptions({ readOnly: true });
    handleSaveDebounce.flush();

    editor.getModel()?.dispose();

    try {
      const content = await readFileText(editorPath);
      const monaco = await monacoImport;
      const uri = monaco.Uri.file(editorPath);

      let model = monaco.editor.getModel(uri);
      if (model) {
        model.setValue(content);
      } else {
        model = monaco.editor.createModel(content, undefined, uri);
      }

      editor.setModel(model);
      updateOutline();
    } finally {
      editor.updateOptions({ readOnly: false });
    }
  };

  const handlePaste = async (event: ClipboardEvent) => {
    const text = event.clipboardData?.getData("text");
    if (text === "") {
      event.preventDefault();
      const res = await paste();

      const range = editorInstance.getSelection();
      const model = editorInstance.getModel();
      if (range && model) {
        model.pushEditOperations(
          [],
          [
            {
              range: range,
              text: `\n#figure(\n  image("${res.path}"),\n  caption: []\n)\n`,
            },
          ],
          () => null
        );
      }
    }
  };

  $: fetchContent(editorInstance, path);
</script>

<div bind:this={divEl} on:paste={handlePaste} class={$$props.class} role="textbox" tabindex="0"></div>

<style>
  div {
    background: var(--color-bg-primary);
  }
</style>
