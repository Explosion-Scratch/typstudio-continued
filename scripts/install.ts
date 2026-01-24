import { $ } from "bun";
import { existsSync } from "fs";

async function run() {
  console.log("🚀 Starting local installation...");

  console.log("📦 Building frontend...");
  await $`bun run build`;

  console.log("🦀 Building Tauri application...");
  // Build for the host architecture
  await $`bun tauri build`;

  console.log("🔍 Locating .app bundle...");
  const appPathResult = await $`find src-tauri/target/release/bundle/macos -name "*.app" -maxdepth 1`.text();
  const appPath = appPathResult.trim();

  if (!appPath || !existsSync(appPath)) {
    console.error("❌ Could not find the built .app bundle.");
    process.exit(1);
  }

  const appName = appPath.split("/").pop();
  const destination = `/Applications/${appName}`;

  console.log(`🚚 Installing to ${destination}...`);
  
  if (existsSync(destination)) {
    console.log("🗑️ Removing existing version...");
    await $`rm -rf ${destination}`;
  }

  await $`cp -R ${appPath} /Applications/`;

  console.log("🔓 Removing quarantine attributes...");
  await $`xattr -d com.apple.quarantine ${destination}`.nothrow();

  console.log(`✅ ${appName} installed successfully to /Applications!`);
}

run().catch(err => {
  console.error("❌ Installation failed:", err);
  process.exit(1);
});
