import { writable, derived } from "svelte/store";
import type { TypstSourceDiagnostic } from "./ipc";

export interface Project {
  root: string;
}

export const project = writable<Project | null>(null);

export interface OutlineItem {
  type: "heading" | "figure" | "table" | "list";
  level: number;
  title: string;
  line: number;
}

export type SidebarTab = "files" | "outline" | "packages";

export interface Shell {
  selectedFile: string | undefined;
  modals: Modal[];
  previewState: PreviewState;
  isInitializing: boolean;
  currentErrors: TypstSourceDiagnostic[];
  activeSidebarTab: SidebarTab;
  documentOutline: OutlineItem[];
  sidebarVisible: boolean;
  currentCompileRequestId: number;
  sidebarWidthPercent: number;
  editorWidthPercent: number;
  viewMode: "both" | "editor" | "preview";
  loadingStage: string;
  loadingProgress: number;
  isOpeningProject: boolean;
}

export interface BaseModal {
  title: string;
}

export interface InputModal extends BaseModal {
  type: "input";
  placeholder?: string;
  callback: (content: string | null) => void;
}

export interface ConfirmModal extends BaseModal {
  type: "confirm";
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel?: () => void;
}

export type Modal = InputModal | ConfirmModal;

export enum PreviewState {
  Idle,
  Compiling,
  CompileError,
}

const createShell = () => {
  const { subscribe, set, update } = writable<Shell>({
    selectedFile: undefined,
    modals: [],
    previewState: PreviewState.Idle,
    isInitializing: true,
    currentErrors: [],
    activeSidebarTab: "files",
    documentOutline: [],
    sidebarVisible: true,
    currentCompileRequestId: 0,
    sidebarWidthPercent: 20,
    editorWidthPercent: 50,
    viewMode: "both",
    loadingStage: "Initializing...",
    loadingProgress: 0,
    isOpeningProject: false,
  });

  let currentRequestId = 0;

  return {
    subscribe,
    selectFile(path: string | undefined) {
      update((shell) => ({
        ...shell,
        selectedFile: path,
      }));
    },
    createModal(modal: Modal) {
      update((shell) => ({
        ...shell,
        modals: [...shell.modals, modal],
      }));
    },
    popModal() {
      update((shell) => {
        const modals = [...shell.modals];
        modals.shift();
        return {
          ...shell,
          modals,
        };
      });
    },
    setPreviewState(previewState: PreviewState) {
      update((shell) => ({ ...shell, previewState }));
    },
    setInitializing(isInitializing: boolean) {
      update((shell) => ({ ...shell, isInitializing }));
    },
    setCurrentErrors(currentErrors: TypstSourceDiagnostic[]) {
      update((shell) => ({ ...shell, currentErrors }));
    },
    setSidebarTab(tab: SidebarTab) {
      update((shell) => ({ ...shell, activeSidebarTab: tab }));
    },
    setDocumentOutline(outline: OutlineItem[]) {
      update((shell) => ({ ...shell, documentOutline: outline }));
    },
    toggleSidebar() {
      update((shell) => ({ ...shell, sidebarVisible: !shell.sidebarVisible }));
    },
    setSidebarVisible(visible: boolean) {
      update((shell) => ({ ...shell, sidebarVisible: visible }));
    },
    nextCompileRequestId(): number {
      currentRequestId++;
      update((shell) => ({ ...shell, currentCompileRequestId: currentRequestId }));
      return currentRequestId;
    },
    getCurrentCompileRequestId(): number {
      return currentRequestId;
    },
    setSidebarWidthPercent(percent: number) {
      update((shell) => ({ ...shell, sidebarWidthPercent: Math.max(10, Math.min(40, percent)) }));
    },
    setEditorWidthPercent(percent: number) {
      update((shell) => ({ ...shell, editorWidthPercent: Math.max(20, Math.min(80, percent)) }));
    },
    setViewMode(mode: "both" | "editor" | "preview") {
      update((shell) => ({ ...shell, viewMode: mode }));
    },
    toggleViewMode() {
      update((shell) => ({
        ...shell,
        viewMode: shell.viewMode === "editor" ? "preview" : "editor",
      }));
    },
    setLoadingStage(stage: string, progress?: number) {
      update((shell) => ({
        ...shell,
        loadingStage: stage,
        loadingProgress: progress ?? shell.loadingProgress,
      }));
    },
    setIsOpeningProject(isOpeningProject: boolean) {
      update((shell) => ({ ...shell, isOpeningProject }));
    },
  };
};

export const shell = createShell();

export interface RecentProject {
  path: string;
  name: string;
  lastOpened: number;
}

const RECENT_PROJECTS_KEY = "typstudio_recent_projects";
const MAX_RECENT_PROJECTS = 10;

const loadRecentProjects = (): RecentProject[] => {
  try {
    const stored = localStorage.getItem(RECENT_PROJECTS_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    console.error("Failed to load recent projects:", e);
  }
  return [];
};

const saveRecentProjects = (projects: RecentProject[]) => {
  try {
    localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(projects));
  } catch (e) {
    console.error("Failed to save recent projects:", e);
  }
};

const createRecentProjects = () => {
  const { subscribe, set, update } = writable<RecentProject[]>(loadRecentProjects());

  return {
    subscribe,
    addProject(path: string) {
      update((projects) => {
        const name = path.split("/").pop() || path;
        const filtered = projects.filter((p) => p.path !== path);
        const newProjects = [
          { path, name, lastOpened: Date.now() },
          ...filtered,
        ].slice(0, MAX_RECENT_PROJECTS);
        saveRecentProjects(newProjects);
        return newProjects;
      });
    },
    removeProject(path: string) {
      update((projects) => {
        const newProjects = projects.filter((p) => p.path !== path);
        saveRecentProjects(newProjects);
        return newProjects;
      });
    },
    clear() {
      set([]);
      saveRecentProjects([]);
    },
  };
};

export const recentProjects = createRecentProjects();
