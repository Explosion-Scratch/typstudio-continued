import { $ } from "bun";
import { readFileSync, writeFileSync } from "fs";

async function run() {
  const args = Bun.argv.slice(2);
  const dryRun = args.includes("--dry-run");
  const versionArg = args.find(arg => !arg.startsWith("--"));
  let newVersion = versionArg;

  if (dryRun) console.log("--- DRY RUN MODE ---");

  const pkgPath = "package.json";
  const tauriPath = "src-tauri/tauri.conf.json";
  const cargoPath = "src-tauri/Cargo.toml";

  const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
  const currentVersion = pkg.version;

  // Validate version if provided
  if (newVersion && (newVersion.startsWith("refs/") || !/^\d+\.\d+\.\d+(-.*)?$/.test(newVersion))) {
    console.log(`Invalid version arg: ${newVersion}. Falling back to package.json version.`);
    newVersion = undefined;
  }

  // Check for gh CLI and auth
  try {
    await $`gh --version`.quiet();
    if (!dryRun && !process.env.GITHUB_ACTIONS) {
      const authStatus = await $`gh auth status`.nothrow().quiet();
      if (authStatus.exitCode !== 0) {
        console.error("gh is not authenticated. Please run 'gh auth login'.");
        process.exit(1);
      }
    }
  } catch (e) {
    console.error("gh CLI is not installed or not in PATH.");
    process.exit(1);
  }

  // Files were defined above

  if (newVersion && newVersion !== currentVersion) {
    console.log(`Updating version from ${currentVersion} to ${newVersion}...`);
    pkg.version = newVersion;
    writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

    const tauri = JSON.parse(readFileSync(tauriPath, "utf-8"));
    tauri.version = newVersion;
    writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n");

    let cargo = readFileSync(cargoPath, "utf-8");
    // Replace only the first occurrence of version = "..." which is the package version
    cargo = cargo.replace(/^version\s*=\s*".*"/m, `version = "${newVersion}"`);
    writeFileSync(cargoPath, cargo);

    console.log("Updated package.json, tauri.conf.json, and Cargo.toml");
  } else {
    newVersion = currentVersion;
    console.log(`Using version: ${newVersion}`);
  }

  const tag = `v${newVersion}`;

  // Get repository information
  const repoUrlResult = await $`git remote get-url origin`.text();
  const repoUrl = repoUrlResult.trim().replace(/\.git$/, "");

  // Get the last tag for the diff link and commit log
  let lastTag = "";
  try {
    lastTag = (await $`git describe --tags --abbrev=0`.text()).trim();
  } catch (e) {
    console.log("No previous tag found, using the first commit.");
    const firstCommit = await $`git rev-list --max-parents=0 HEAD`.text();
    lastTag = firstCommit.trim() || "HEAD";
  }

  const diffLink = `${repoUrl}/compare/${lastTag}...${tag}`;
  const commits = (await $`git log ${lastTag}..HEAD --oneline --no-merges`.text()).trim();

  const releaseNotes = `## Version ${newVersion}

### Changes:
${commits || "No changes found."}

**Full Diff**: ${diffLink}`;

  console.log("\nGenerated Release Notes:");
  console.log("------------------------");
  console.log(releaseNotes);
  console.log("------------------------\n");

  if (!dryRun) {
    console.log("Building frontend...");
    await $`bun run build`;

    const skipBuildCmd = JSON.stringify({
      version: newVersion,
      build: { beforeBuildCommand: "" }
    });

    // Write a temporary config to avoid shell interpolation/parsing issues
    const tempConfigPath = "src-tauri/temp.config.json";
    writeFileSync(tempConfigPath, skipBuildCmd);

    console.log("Building for Mac x86_64...");
    await $`bun tauri build --target x86_64-apple-darwin --config ${tempConfigPath}`;

    console.log("Building for Mac ARM64...");
    await $`bun tauri build --target aarch64-apple-darwin --config ${tempConfigPath}`;

    // Cleanup temp config
    try {
      await $`rm ${tempConfigPath}`;
    } catch (e) {}
  }

  // Collect artifacts
  console.log("Locating artifacts...");
  const artifacts = (await $`find src-tauri/target -name "*.dmg"`.text())
    .split("\n")
    .map(f => f.trim())
    .filter(f => f && f.includes("release") && (f.includes("x86_64") || f.includes("aarch64") || f.includes("universal")));

  if (artifacts.length === 0 && !dryRun) {
    console.error("No DMG artifacts found!");
    process.exit(1);
  }

  console.log(`Found artifacts:\n${artifacts.map(a => ` - ${a}`).join("\n")}`);

  if (dryRun) {
    console.log(`[DRY RUN] Would create release ${tag} with notes and ${artifacts.length} artifacts.`);
  } else {
    console.log(`Creating GitHub release ${tag}...`);
    await $`gh release create ${tag} --title "${tag}" --notes "${releaseNotes}" ${artifacts}`;
    console.log("Release published successfully!");
  }
}

run().catch(err => {
  console.error("Release failed:", err);
  process.exit(1);
});
