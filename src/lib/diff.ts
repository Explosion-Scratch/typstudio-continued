import { writable } from 'svelte/store';

export interface DiffStats {
    added: number;
    removed: number;
}

export const diffStats = writable<DiffStats>({ added: 0, removed: 0 });
export const showDiffEditor = writable<boolean>(false);
