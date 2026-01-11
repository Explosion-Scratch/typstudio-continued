import { invoke } from "@tauri-apps/api";

export interface TypstCompileEvent {
  document: TypstDocument | null;
  diagnostics: TypstSourceDiagnostic[] | null;
}

export interface TypstDocument {
  pages: number;
  hash: string;
  width: number;
  height: number;
}

export type TypstDiagnosticSeverity = "error" | "warning";

export interface TypstSourceDiagnostic {
  range: { start: number; end: number };
  severity: TypstDiagnosticSeverity;
  message: string;
  hints: string[];
}

export interface TypstRenderResponse {
  image: string;
  width: number;
  height: number;
  nonce: number;
}

export enum TypstCompletionKind {
  Syntax = 1,
  Function = 2,
  Parameter = 3,
  Constant = 4,
  Symbol = 5,
  Type = 6,
}

export interface TypstCompletion {
  kind: TypstCompletionKind;
  label: string;
  apply: string | null;
  detail: string | null;
}

export interface TypstCompleteResponse {
  offset: number;
  completions: TypstCompletion[];
}

export const compile = (path: string, content: string, requestId: number): Promise<TypstRenderResponse> =>
  invoke<TypstRenderResponse>("typst_compile", { path, content, requestId });

export const render = (page: number, scale: number, nonce: number): Promise<TypstRenderResponse> =>
  invoke<TypstRenderResponse>("typst_render", { page, scale, nonce });

export const autocomplete = (
  path: string,
  content: string,
  offset: number,
  explicit: boolean
): Promise<TypstCompleteResponse> =>
  invoke<TypstCompleteResponse>("typst_autocomplete", { path, content, offset, explicit });

export interface TypstJump {
  filepath: string;
  start: [number, number] | null; // line, column (1-indexed)
  end: [number, number] | null;
  text?: string;
  offset?: number;
  node_kind?: string;
}

export const jump = (page: number, x: number, y: number): Promise<TypstJump | null> =>
  invoke<TypstJump | null>("typst_jump", { page, x, y });

export interface InstalledPackage {
  namespace: string;
  name: string;
  version: string;
}

export const listPackages = (): Promise<InstalledPackage[]> =>
  invoke<InstalledPackage[]>("typst_list_packages");

export const deletePackage = (namespace: string, name: string, version: string): Promise<void> =>
  invoke("typst_delete_package", { namespace, name, version });

export const installPackage = (spec: string): Promise<void> =>
  invoke("typst_install_package", { spec });

export interface TypstDocumentPosition {
  page: number;
  x: number;
  y: number;
  text?: string;
  node_kind?: string;
}

export const jumpFromCursor = (
  path: string,
  content: string,
  byteOffset: number
): Promise<TypstDocumentPosition | null> =>
  invoke<TypstDocumentPosition | null>("typst_jump_from_cursor", { path, content, byteOffset });

