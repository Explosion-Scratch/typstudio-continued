<script>
  import "../app.css";
  import { onMount } from "svelte";
  import { recentProjects, shell, project } from "$lib/stores";
  import { invoke } from "@tauri-apps/api/core";

  let { children } = $props();

  $effect(() => {
    const projects = $recentProjects;
    const isProjectOpen = !!$project;
    console.log("[Layout] Updating menu state (effect):", { project_count: projects.length, isProjectOpen });
    invoke("update_menu_state", { projects, isProjectOpen }).catch((e) => console.error("Failed to update menu state:", e));
  });

  onMount(async () => {
    // Force initial update
    const projects = $recentProjects;
    const isProjectOpen = !!$project;
    console.log("[Layout] Updating menu state (mount):", { project_count: projects.length, isProjectOpen });
    invoke("update_menu_state", { projects, isProjectOpen }).catch((e) => console.error("Failed to update menu state:", e));
    
    // Listen for export menu events
    const { listen } = await import("@tauri-apps/api/event");
    
    await listen("menu_export_svg", (event) => {
        const path = event.payload;
        if (typeof path === "string") {
            invoke("export_svg", { path }).catch(console.error);
        }
    });

    await listen("menu_export_pdf", (event) => {
        const path = event.payload;
        if (typeof path === "string") {
            invoke("export_pdf", { path }).catch(console.error);
        }
    });
    
    await listen("menu_export_png", (event) => {
        const path = event.payload;
        if (typeof path === "string") {
            invoke("export_png", { path }).catch(console.error);
        }
    });

    await listen("menu_open_recent", async (event) => {
        const index = event.payload;
        if (typeof index === "number") {
             const project = $recentProjects[index];
             if (project) {
                 shell.setIsOpeningProject(true);
                 shell.setLoadingStage(`Opening ${project.name}...`, 0);
                 try {
                     await invoke("open_project", { path: project.path });
                 } catch (e) {
                     console.error("Failed to open recent project", e);
                     shell.setIsOpeningProject(false);
                 }
             }
        }
    });
    
    await listen("menu_clear_recent", () => {
        recentProjects.clear();
    });

    await listen("menu_new_file", () => {
        if (!$project) return;
        shell.createModal({
            type: "input",
            title: "New File",
            placeholder: "filename.typ",
            callback: async (filename) => {
                if (!filename) return;
                try {
                    // Use invoke("fs_create_file") or imported createFile if available in context, 
                    // but prefer invoke for raw simplicity if not imported.
                    // However, we can dynamically import or just use invoke.
                    // To keep it simple and consistent with other event handlers here:
                    const { createFile, listDir } = await import("$lib/ipc");
                    
                    // Determine where to put the file? 
                    // Usually relative to selected file or root.
                    // For now, let's just put it in root or relative to current selection if possible?
                    // shell.selectedFile gives absolute path.
                    // Let's default to root for now if no selection, or parent of selected.
                    
                    let parent = "/";
                    const selected = $shell.selectedFile;
                    if (selected) {
                        const parts = selected.split("/");
                        parts.pop();
                        parent = parts.join("/");
                        if (!parent) parent = "/";
                    }
                    
                    const path = parent === "/" ? `/${filename}` : `${parent}/${filename}`;
                    await createFile(path);
                    
                    // Select the new file
                    shell.selectFile(path);
                } catch (e) {
                    console.error("Failed to create file:", e);
                }
            }
        });
    });
  });
</script>

{@render children()}