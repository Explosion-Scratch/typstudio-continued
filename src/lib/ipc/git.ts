import { invoke } from "@tauri-apps/api/core";

export async function getOriginalFileContent(path: string): Promise<string> {
  try {
    return await invoke<string>("git_read_original_file", { path });
  } catch (e) {
    console.error("Failed to get original file content", e);
    return "";
  }
}
