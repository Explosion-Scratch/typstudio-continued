<script>
  import "../app.css";
  import { onMount } from "svelte";
  import { recentProjects, shell } from "$lib/stores";
  import { invoke } from "@tauri-apps/api/core";

  let { children } = $props();

  onMount(async () => {
    const projects = $recentProjects;
    if (projects.length > 0) {
      invoke("update_recent_menu", { projects }).catch(console.error);
    }
    
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
  });
</script>

{@render children()}