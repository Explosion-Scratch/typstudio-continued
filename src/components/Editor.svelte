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
  import { diffStats, showDiffEditor } from "$lib/diff";
  import { computeDiffStats } from "$lib/diff-utils";
  import { ArrowLeft } from "$lib/icons";

  type ICodeEditor = editor.ICodeEditor;
  type IDiffEditor = editor.IDiffEditor;
  type IModelContentChangedEvent = editor.IModelContentChangedEvent;
  type IModelChangedEvent = editor.IModelChangedEvent;
  type IMarkerData = editor.IMarkerData;

  let divEl: HTMLDivElement;
  let editorInstance: ICodeEditor | IDiffEditor;
  let currentModelUri: any;
  let cachedOriginalContent: string = "";
  const monacoImport = import("monaco-editor");

  export let path: string;
  export let isVisible: boolean = true;

  $: if (isVisible && editorInstance) {
    const pending = $pendingScroll;
    if (pending.source === "preview" && pending.line) {
      const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
      const currentPath = model?.uri.path;
      if (!pending.filepath || pending.filepath === currentPath) {
        scrollToPosition(pending.line);
      }
      pendingScroll.update((p) => ({ ...p, source: null }));
    }
  }

  let isTyping = false;
  let isJumping = false;
  let lastCompileRequestId = 0;
  let lastDiagnostics: TypstSourceDiagnostic[] = [];


  const applyMarkers = (diagnostics: TypstSourceDiagnostic[]) => {
    // If diff editor, get modified model. If code editor, get model directly.
    const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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
    const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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
      const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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

  const handleCompile = async (overridePreviewFile?: string) => {
    const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
    if (model) {
      const filePath = model.uri.path;
      if (!filePath.endsWith(".typ")) return;

      const requestId = shell.nextCompileRequestId();
      lastCompileRequestId = requestId;
      shell.setPreviewState(PreviewState.Compiling);
      
      const previewFile = overridePreviewFile ?? $shell.previewFile;
      const mainPath = previewFile !== filePath ? previewFile : undefined;
      await compile(model.uri.path, model.getValue(), requestId, mainPath);
    }
  };

  const handleCompileDebounced = debounce(handleCompile, 50, { maxWait: 200 });

  const handleSave = () => {
    // Return promise to allow awaiting save completion
    const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
    if (model) {
      return writeFileText(model.uri.path, model.getValue()).then(() => {
      });
    }
    return Promise.resolve();
  };

  const handleSaveDebounce = debounce(handleSave, 1000, { maxWait: 5000 });

  const handleCursorJump = debounce(async () => {
    if (editorInstance) {
      // Use type guard directly for narrowing
      const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
      const position = isDiffEditor(editorInstance) ? editorInstance.getModifiedEditor().getPosition() : editorInstance.getPosition();
      
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
      const editor = isDiffEditor(editorInstance) ? editorInstance.getModifiedEditor() : editorInstance;
      scrollEditorToCenterLine(editor, line, column);
      editorInstance.focus();

      setTimeout(() => {
        isJumping = false;
      }, 1000);
    }
  };

  export const getCursorPosition = () => {
    if (editorInstance) {
      const position = isDiffEditor(editorInstance) ? editorInstance.getModifiedEditor().getPosition() : editorInstance.getPosition();
      const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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

  // Helper to check editor type
  const isDiffEditor = (e: any): e is IDiffEditor => {
      return e && typeof e.getModifiedEditor === 'function';
  }

  onMount(() => {
    let cleanup: (() => void)[] = [];
    const syncPreviewFromScroll = async () => {
      if (!editorInstance || isJumping) return;

      const editor = isDiffEditor(editorInstance) ? editorInstance.getModifiedEditor() : editorInstance;
      const target = await getEditorToPreviewTarget(editor);

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
    
    // Debounced stats update
    const updateDiffStatsDebounced = debounce((content: string, original: string) => {
         const stats = computeDiffStats(original, content);
         diffStats.set(stats);
    }, 1000);

    const createEditor = async () => {
        if (editorInstance) {
            editorInstance.dispose();
        }
        
        const monaco = await monacoImport;
        
        // Common options
        const options: editor.IStandaloneEditorConstructionOptions = {
          lineHeight: 1.8,
          automaticLayout: true,
          readOnly: false,
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
        };

        if ($showDiffEditor) {
             editorInstance = monaco.editor.createDiffEditor(divEl, {
                 ...options,
                 readOnly: false, 
                 renderSideBySide: false,
             });
        } else {
             editorInstance = monaco.editor.create(divEl, options);
        }
        
        registerListeners(editorInstance);
        await fetchContent(editorInstance, path);
    }
    
    // Register listeners for either editor type
    const registerListeners = (instance: ICodeEditor | IDiffEditor) => {
        const modifiedEditor = isDiffEditor(instance) ? instance.getModifiedEditor() : instance;

        modifiedEditor.onDidChangeModelContent(async () => {
             clearMarkersWhileTyping();
             markTypingDone();
             handleCompileDebounced();
             handleSaveDebounce();
             updateOutlineDebounced();
             
             // Update diff stats
             const model = modifiedEditor.getModel();
             if (model) {
                 const content = model.getValue();
                 updateDiffStatsDebounced(content, cachedOriginalContent);
             }
        });
        
        // Mouse and Cursor
        modifiedEditor.onMouseDown(() => handleCursorJump());
        modifiedEditor.onDidChangeCursorPosition(() => {
             const pos = getCursorPosition();
             if (pos) appWindow.emit("editor_cursor_changed", pos);
        });
        modifiedEditor.onDidScrollChange((e) => {
             if (e.scrollTopChanged) syncPreviewFromScrollDebounced();
        });
    }

    (async () => {
      const EditorWorker = await import("monaco-editor/esm/vs/editor/editor.worker?worker");
      await initMonaco;

      (self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
        getWorker: function (_moduleId: unknown, label: string) {
          return new EditorWorker.default();
        },
      };

      await createEditor();

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

      const unsubscribeTriggerCompile = await appWindow.listen<{ previewFile?: string }>(
        "trigger_compile",
        ({ payload }) => handleCompile(payload?.previewFile),
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
          const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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
            const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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
          const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
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
    
    // React to showDiffEditor changes
    const unsubDiffStore = showDiffEditor.subscribe(async (val) => {
         if (editorInstance) {
              await createEditor();
         }
    });
    cleanup.push(unsubDiffStore);

    return () => {
      cleanup.forEach((fn) => fn());
      if (editorInstance) editorInstance.dispose();
      syncPreviewFromScrollDebounced.cancel();
      updateDiffStatsDebounced.cancel(); 
    };
  });

  const fetchContent = async (editor: ICodeEditor | IDiffEditor, editorPath: string) => {
    if (!editor) return;
    editor.updateOptions({ readOnly: true });
    
    // Ensure any pending save is flushed and completed before we potentially switch/dispose models
    // or read from disk into a new model
    if (handleSaveDebounce) {
       await handleSaveDebounce.flush();
    }
    
    try {
      const monaco = await monacoImport;
      const uri = monaco.Uri.file(editorPath);
      
      const [content, originalContent] = await Promise.all([
        readFileText(editorPath),
        getOriginalFileContent(editorPath)
      ]);
      
      cachedOriginalContent = originalContent;

      // Update stats immediately
      const stats = computeDiffStats(originalContent, content);
      diffStats.set(stats);
      
      // Get or Create Modified Model
      let modifiedModel = monaco.editor.getModel(uri);
      if (modifiedModel) {
          modifiedModel.setValue(content);
      } else {
          modifiedModel = monaco.editor.createModel(content, undefined, uri);
      }
      
      if (isDiffEditor(editor)) {
           // Diff Editor Setup
           const originalUri = uri.with({ scheme: 'original' });
           // NOTE: We should check if original model exists to avoid collision/leak?
           let originalModel = monaco.editor.getModel(originalUri);
           if (originalModel) {
               originalModel.setValue(originalContent || "");
           } else {
               originalModel = monaco.editor.createModel(originalContent || "", undefined, originalUri);
           }
           
           editor.setModel({
               original: originalModel,
               modified: modifiedModel
           });
      } else {
           // Standard Editor Setup
            editor.setModel(modifiedModel);
      }

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
      
      const range = isDiffEditor(editorInstance) ? editorInstance.getModifiedEditor().getSelection() : editorInstance.getSelection();
      const model = isDiffEditor(editorInstance) ? editorInstance.getModel()?.modified : editorInstance.getModel();
      
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

<div class="editor-wrapper {$$props.class}">
    <div
      bind:this={divEl}
      on:paste={handlePaste}
      class="editor-div"
      role="textbox"
      tabindex="0"
    ></div>

</div>

<style>
  .editor-wrapper {
      position: relative;
      width: 100%;
      height: 100%;
  }

  .editor-div {
    background: var(--color-bg-primary);
    width: 100%;
    height: 100%;
  }
  
</style>
