<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { editor } from "monaco-editor";
  import debounce from "lodash/debounce";
  import { initMonaco } from "../lib/editor/monaco";
  import type { TypstCompileEvent, TypstSourceDiagnostic } from "$lib/ipc";
  import {
    compile,
    readFileText,
    writeFileText,
    jumpFromCursor,
    getOriginalFileContent,
  } from "$lib/ipc";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  const appWindow = getCurrentWindow();
  import { paste } from "$lib/ipc/clipboard";
  import { PreviewState, shell, pendingScroll } from "$lib/stores";
  import { extractOutlineFromSource } from "$lib/outline";
  import { getEditorToPreviewTarget, scrollEditorToCenterLine } from "$lib/scroll";

  type ICodeEditor = editor.ICodeEditor;
  type IDiffEditor = editor.IDiffEditor;
  type IModelContentChangedEvent = editor.IModelContentChangedEvent;
  type IModelChangedEvent = editor.IModelChangedEvent;
  type IMarkerData = editor.IMarkerData;
  type IRange = editor.IRange;

  let divEl: HTMLDivElement;
  let editorInstance: IDiffEditor;
  const monacoImport = import("monaco-editor");

  export let path: string;
  export let isVisible: boolean = true;

  $: if (isVisible && editorInstance) {
    const pending = $pendingScroll;
    if (pending.source === "preview" && pending.line) {
      scrollToPosition(pending.line);
      pendingScroll.update((p) => ({ ...p, source: null }));
    }
  }

  let isTyping = false;
  let isJumping = false;
  let lastCompileRequestId = 0;
  let lastDiagnostics: TypstSourceDiagnostic[] = [];


  const applyMarkers = (diagnostics: TypstSourceDiagnostic[]) => {
    const model = editorInstance?.getModel()?.modified;
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
    const model = editorInstance?.getModel()?.modified;
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
      const model = editorInstance?.getModel()?.modified;
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
    const model = editorInstance?.getModel()?.modified;
    if (model) {
      const filePath = model.uri.path;
      if (!filePath.endsWith(".typ")) return;

      const requestId = shell.nextCompileRequestId();
      lastCompileRequestId = requestId;
      shell.setPreviewState(PreviewState.Compiling);
      await compile(model.uri.path, model.getValue(), requestId);
    }
  };

  const handleSave = () => {
    const model = editorInstance.getModel()?.modified;
    if (model) {
      writeFileText(model.uri.path, model.getValue()).then(() => {
        // updateGitDecorations(); // Removed, handled natively
      });
    }
  };

  const handleSaveDebounce = debounce(handleSave, 1000, { maxWait: 5000 });

  const handleCursorJump = debounce(async () => {
    if (editorInstance) {
      const model = editorInstance.getModel()?.modified;
      const position = editorInstance.getPosition();
      if (model && position && model.uri.path.endsWith(".typ")) {
        const content = model.getValue();
        const offset = model.getOffsetAt(position);

        const byteOffset = new TextEncoder().encode(content.substring(0, offset)).length;

        try {
          const result = await jumpFromCursor(model.uri.path, content, byteOffset);
          if (result) {
            if ($shell.viewMode === "both" || isVisible) {
              appWindow.emit("scroll_to_position_in_preview", result);
            }

            if ($shell.viewMode === "editor") {
              pendingScroll.update((p) => ({
                ...p,
                source: "editor",
                preview: result,
              }));
            }
          }
        } catch (e) {
          console.error("Failed to jump from cursor:", e);
        }
      }
    }
  }, 300);

  export const scrollToPosition = (line: number, column: number = 1) => {
    if (editorInstance) {
      isJumping = true;
      scrollEditorToCenterLine(editorInstance.getModifiedEditor(), line, column);
      editorInstance.focus();

      setTimeout(() => {
        isJumping = false;
      }, 1000);
    }
  };

  export const getCursorPosition = () => {
    if (editorInstance) {
      const position = editorInstance.getPosition();
      const model = editorInstance.getModel()?.modified;
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
    const syncPreviewFromScroll = async () => {
      if (!editorInstance || isJumping) return;

      const target = await getEditorToPreviewTarget(editorInstance.getModifiedEditor());

      if (target) {
        if ($shell.viewMode === "editor") {
          pendingScroll.update((p) => ({
            ...p,
            source: "editor",
            preview: target,
          }));
        } else if ($shell.viewMode === "both") {
          appWindow.emit("scroll_to_position_in_preview", { ...target, flash: false });
        }
      }
    };

    const syncPreviewFromScrollDebounced = debounce(syncPreviewFromScroll, 10);

    (async () => {
      const EditorWorker = await import("monaco-editor/esm/vs/editor/editor.worker?worker");
      await initMonaco;

      (self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
        getWorker: function (_moduleId: unknown, label: string) {
          return new EditorWorker.default();
        },
      };

      editorInstance = (await monacoImport).editor.createDiffEditor(divEl, {
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
        renderSideBySide: false,
        scrollbar: {
          vertical: "auto",
          horizontal: "auto",
          verticalScrollbarSize: 8,
          horizontalScrollbarSize: 8,
        },
      });

      editorInstance.onDidChangeModel(() => {
        handleCompile();
        updateOutline();
      });

      editorInstance.onDidUpdateDiff(() => {
        // Handle logic when diff updates if needed
      });

      // We need to listen to the *modified* editor for normal typing events
      const modifiedEditor = editorInstance.getModifiedEditor();

      modifiedEditor.onDidChangeModelContent(() => {
        clearMarkersWhileTyping();
        markTypingDone();
        handleCompile();
        handleSaveDebounce();
        updateOutlineDebounced();
      });

      editorInstance.onDidUpdateDiff(() => {
          // Might need to update outline or decorations if diff changes, but content change usually covers it.
      });

      // Mouse events on the modified editor
      modifiedEditor.onMouseDown(() => {
        handleCursorJump();
      });

      // Cursor position on modified editor
      modifiedEditor.onDidChangeCursorPosition(() => {
        const pos = getCursorPosition();
        if (pos) appWindow.emit("editor_cursor_changed", pos);
      });

      // Scroll changes on modified editor
      modifiedEditor.onDidScrollChange((e) => {
        if (e.scrollTopChanged) syncPreviewFromScrollDebounced();
      });

      const unsubscribeCompile = await appWindow.listen<TypstCompileEvent>(
        "typst_compile",
        ({ payload }) => {
          const { document, diagnostics } = payload;
          lastDiagnostics = diagnostics || [];

          if (!isTyping) applyMarkers(lastDiagnostics);
          shell.setPreviewState(document ? PreviewState.Idle : PreviewState.CompileError);
        },
      );
      cleanup.push(unsubscribeCompile);

      const unsubscribeJumpTo = await appWindow.listen<{ line: number; column?: number }>(
        "jump_to_position",
        ({ payload }) => {
          scrollToPosition(payload.line, payload.column || 1);
        },
      );
      cleanup.push(unsubscribeJumpTo);

      const unsubscribeTriggerCompile = await appWindow.listen("trigger_compile", () =>
        handleCompile(),
      );
      cleanup.push(unsubscribeTriggerCompile);

      const unsubscribeSave = await appWindow.listen("menu_save", () => handleSave());
      cleanup.push(unsubscribeSave);

      const unsubscribeSaveAll = await appWindow.listen("menu_save_all", () => handleSave());
      cleanup.push(unsubscribeSaveAll);

      const unsubscribeReplaceRange = await appWindow.listen<{
        startLine: number;
        endLine: number;
        text: string;
      }>("replace_range", ({ payload }) => {
        if (editorInstance) {
          const model = editorInstance.getModel()?.modified;
          if (model) {
            const range = {
              startLineNumber: payload.startLine,
              startColumn: 1,
              endLineNumber: payload.endLine,
              endColumn: model.getLineMaxColumn(payload.endLine),
            };
            model.pushEditOperations([], [{ range: range, text: payload.text }], () => null);
          }
        }
      });
      cleanup.push(unsubscribeReplaceRange);

      const unsubscribeDeleteRange = await appWindow.listen<{ startLine: number; endLine: number }>(
        "delete_range",
        ({ payload }) => {
          if (editorInstance) {
            const model = editorInstance.getModel()?.modified;
            if (model) {
              const range = {
                startLineNumber: payload.startLine,
                startColumn: 1,
                endLineNumber: payload.endLine,
                endColumn: model.getLineMaxColumn(payload.endLine),
              };
              model.pushEditOperations([], [{ range: range, text: "" }], () => null);
            }
          }
        },
      );
      cleanup.push(unsubscribeDeleteRange);

      const unsubscribeExtractSection = await appWindow.listen<{
        startLine: number;
        endLine: number;
        filename: string;
      }>("extract_section", async ({ payload }) => {
        if (editorInstance) {
          const model = editorInstance.getModel()?.modified;
          if (model) {
            const range = {
              startLineNumber: payload.startLine,
              startColumn: 1,
              endLineNumber: payload.endLine,
              endColumn: model.getLineMaxColumn(payload.endLine),
            };
            const content = model.getValueInRange(range);
            const currentPath = model.uri.path;
            const parentDir = currentPath.substring(0, currentPath.lastIndexOf("/") + 1);
            const newFilePath = parentDir + payload.filename;

            try {
              await writeFileText(newFilePath, content);
              model.pushEditOperations(
                [],
                [{ range: range, text: `#include "${payload.filename}"` }],
                () => null,
              );
            } catch (err) {
              console.error("Failed to extract section:", err);
            }
          }
        }
      });
      cleanup.push(unsubscribeExtractSection);
    })();

    return () => {
      cleanup.forEach((fn) => fn());
      if (editorInstance) editorInstance.dispose();
      syncPreviewFromScrollDebounced.cancel();
    };
  });

  const fetchContent = async (editor: IDiffEditor, editorPath: string) => {
    if (!editor) return;
    editor.updateOptions({ readOnly: true });
    handleSaveDebounce.flush();
    const currentModels = editor.getModel();
    if (currentModels) {
      currentModels.original.dispose();
      currentModels.modified.dispose();
    }

    try {
      const [content, originalContent] = await Promise.all([
        readFileText(editorPath),
        getOriginalFileContent(editorPath)
      ]);
      
      const monaco = await monacoImport;
      const uri = monaco.Uri.file(editorPath);
      
      // Original model (from git)
      const originalModel = monaco.editor.createModel(originalContent || "", undefined, uri.with({ scheme: 'original' })); // Use distinct URI
      
      // Modified model (current content)
      let modifiedModel = monaco.editor.getModel(uri);
      if (modifiedModel) {
        modifiedModel.setValue(content);
      } else {
        modifiedModel = monaco.editor.createModel(content, undefined, uri);
      }
      
      editor.setModel({
        original: originalModel,
        modified: modifiedModel
      });

      updateOutline();
      // updateGitDecorations(); // Removed
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
      const model = editorInstance.getModel()?.modified;
      if (range && model) {
        model.pushEditOperations(
          [],
          [
            {
              range: range,
              text: `\n#figure(\n  image("${res.path}"),\n  caption: []\n)\n`,
            },
          ],
          () => null,
        );
      }
    }
  };

  $: fetchContent(editorInstance, path);
</script>

<div
  bind:this={divEl}
  on:paste={handlePaste}
  class={$$props.class}
  role="textbox"
  tabindex="0"
></div>

<style>
  div {
    background: var(--color-bg-primary);
  }


</style>
