<script lang="ts">
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

    const handleKeydown = (e: KeyboardEvent) => {
      const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
      const modifier = isMac ? e.metaKey : e.ctrlKey;

      if (modifier && e.key === 'p') {
        e.preventDefault();
        if ($project) {
          shell.createModal({
            type: "quick_open",
            title: "Quick Open"
          });
        }
      }
    };

    window.addEventListener('keydown', handleKeydown);
    
    // Listen for export menu events
    const { listen } = await import("@tauri-apps/api/event");
    
    const unlistenFns = [
        await listen("menu_export_svg", (event) => {
            const path = event.payload;
            if (typeof path === "string") {
                invoke("export_svg", { path }).catch(console.error);
            }
        }),

        await listen("menu_export_pdf", (event) => {
            const path = event.payload;
            if (typeof path === "string") {
                invoke("export_pdf", { path }).catch(console.error);
            }
        }),
        
        await listen("menu_export_png", (event) => {
            const path = event.payload;
            if (typeof path === "string") {
                invoke("export_png", { path }).catch(console.error);
            }
        }),

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
        }),
        
        await listen("menu_clear_recent", () => {
            recentProjects.clear();
        }),

        await listen("menu_new_file", () => {
            if (!$project) return;
            shell.createModal({
                type: "input",
                title: "New File",
                placeholder: "filename.typ",
                callback: async (filename) => {
                    if (!filename) return;
                    try {
                        const { createFile } = await import("$lib/ipc");
                        
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
                        
                        shell.selectFile(path);
                    } catch (e) {
                        console.error("Failed to create file:", e);
                    }
                }
            });
        })
    ];

    return () => {
      window.removeEventListener('keydown', handleKeydown);
      unlistenFns.forEach(fn => fn());
    };
  });
</script>

{@render children()}
