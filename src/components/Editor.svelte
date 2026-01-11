<script lang="ts">
  import { onMount } from "svelte";
  import type { editor } from "monaco-editor";
  import debounce from "lodash/debounce";

  import { initMonaco } from "../lib/editor/monaco";
  import type { TypstCompileEvent, TypstSourceDiagnostic } from "../lib/ipc";
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
  let lastDiagnostics: TypstSourceDiagnostic[] = [];

  const applyMarkers = (diagnostics: TypstSourceDiagnostic[]) => {
    const model = editorInstance?.getModel();
    if (model) {
      import("monaco-editor").then((m) => {
        const markers: IMarkerData[] = diagnostics.map(({ range, severity, message, hints }) => {
          const start = model.getPositionAt(range.start);
          const end = model.getPositionAt(range.end);
          return {
            startLineNumber: start.lineNumber,
            startColumn: start.column,
            endLineNumber: end.lineNumber,
            endColumn: end.column,
            message: message + "\n" + hints.map((hint: string) => `hint: ${hint}`).join("\n"),
            severity: severity === "error" ? m.MarkerSeverity.Error : m.MarkerSeverity.Warning,
          };
        });
        m.editor.setModelMarkers(model, "owner", markers);
      });
    }
  };

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
    applyMarkers(lastDiagnostics);
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
      console.log(`[Frontend] Triggering compile request_id: ${requestId}`);
      shell.setPreviewState(PreviewState.Compiling);
      await compile(model.uri.path, model.getValue(), requestId);
      console.log(`[Frontend] Sent compile command request_id: ${requestId}`);
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
        const content = model.getValue();
        const offset = model.getOffsetAt(position);
        
        // Use Monaco API to get context text around the cursor
        const startLine = Math.max(1, position.lineNumber - 1);
        const endLine = Math.min(model.getLineCount(), position.lineNumber + 1);
        const contextText = model.getValueInRange({
          startLineNumber: startLine,
          startColumn: 1,
          endLineNumber: endLine,
          endColumn: model.getLineMaxColumn(endLine)
        });

        // Calculate byte offset for Typst (UTF-8)
        const byteOffset = new TextEncoder().encode(content.substring(0, offset)).length;

        try {
          const result = await jumpFromCursor(model.uri.path, content, byteOffset);
          if (result) {
            console.log("Jump Target Received (Editor -> Preview):", {
              source: {
                line: position.lineNumber,
                column: position.column,
                offset: offset,
                byteOffset: byteOffset,
                context: contextText.trim(),
                kind: result.node_kind
              },
              target: {
                page: result.page + 1,
                x: Math.round(result.x),
                y: Math.round(result.y)
              }
            });
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
    let lastSyncScrollTop = 0;
    
    const syncPreviewFromScroll = async () => {
       if (!editorInstance) return;
       const scrollTop = editorInstance.getScrollTop();
       const delta = Math.abs(scrollTop - lastSyncScrollTop);
       
       if (delta > 200) {
           const model = editorInstance.getModel();
           if (!model) return;
           
           const ranges = editorInstance.getVisibleRanges();
           if (ranges.length === 0) return;
           
           const centerLine = Math.floor((ranges[0].startLineNumber + ranges[0].endLineNumber) / 2);
           
           const pos = { lineNumber: centerLine, column: 1 };
           const offset = model.getOffsetAt(pos);
           const content = model.getValue();
           const byteOffset = new TextEncoder().encode(content.substring(0, offset)).length;
           
           try {
              const result = await jumpFromCursor(model.uri.path, content, byteOffset);
              if (result) {
                appWindow.emit("scroll_to_position_in_preview", { ...result, flash: false });
                lastSyncScrollTop = scrollTop;
              }
           } catch (err) {
              console.error("Failed to sync preview from scroll:", err);
           }
       }
    };
    
    const syncPreviewFromScrollDebounced = debounce(syncPreviewFromScroll, 500);

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

      editorInstance.onMouseDown(() => {
        handleCursorJump();
      });

      editorInstance.onDidChangeCursorPosition(() => {
        const pos = getCursorPosition();
        if (pos) {
          appWindow.emit("editor_cursor_changed", pos);
        }
      });

      editorInstance.onDidScrollChange((e) => {
        if (e.scrollTopChanged) {
           syncPreviewFromScrollDebounced();
        }
      });

      const unsubscribeCompile = await appWindow.listen<TypstCompileEvent>("typst_compile", ({ payload }) => {
        console.log("[Frontend] Received typst_compile event");
        const { document, diagnostics } = payload;
        
        lastDiagnostics = diagnostics || [];
        
        if (!isTyping) {
          applyMarkers(lastDiagnostics);
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
      const unsubscribeTriggerCompile = await appWindow.listen("trigger_compile", () => {
        handleCompile();
      });
      cleanup.push(unsubscribeTriggerCompile);
    })();

    return () => {
      cleanup.forEach((fn) => fn());
      if (editorInstance) editorInstance.dispose();
      syncPreviewFromScrollDebounced.cancel();
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
