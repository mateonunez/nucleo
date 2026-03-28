#!/usr/bin/env node

/**
 * nucleo "scaffold" plugin — scaffold projects, layouts, and components from templates.
 *
 * Usage:
 *   nucleo plugins scaffold list                           # list available templates
 *   nucleo plugins scaffold create <name> <template>       # scaffold a project
 *   nucleo plugins scaffold create <name> <template> --dry-run
 *
 * Templates directory resolution (first match wins):
 *   1. <PREFIX>_TEMPLATES_DIR env var
 *   2. ./templates/  (cwd)
 *   3. <PLUGIN_DIR>/../../templates/  (relative to plugin)
 */

const fs = require("fs");
const path = require("path");

const PREFIX = process.env.CLI_ENV_PREFIX || "NUCLEO";

// ---------------------------------------------------------------------------
// Template directory resolution
// ---------------------------------------------------------------------------

function resolveTemplatesDir() {
  const envVar = `${PREFIX}_TEMPLATES_DIR`;
  const fromEnv = process.env[envVar];
  if (fromEnv && fs.existsSync(fromEnv) && fs.statSync(fromEnv).isDirectory()) {
    return fromEnv;
  }

  const cwd = path.join(process.cwd(), "templates");
  if (fs.existsSync(cwd) && fs.statSync(cwd).isDirectory()) {
    return cwd;
  }

  const pluginDir = process.env[`${PREFIX}_PLUGIN_DIR`] || __dirname;
  const repoTemplates = path.resolve(pluginDir, "..", "..", "templates");
  if (fs.existsSync(repoTemplates) && fs.statSync(repoTemplates).isDirectory()) {
    return repoTemplates;
  }

  return null;
}

function listTemplates(dir) {
  return fs
    .readdirSync(dir)
    .filter((name) => {
      if (name.startsWith(".")) return false;
      return fs.statSync(path.join(dir, name)).isDirectory();
    })
    .sort();
}

// ---------------------------------------------------------------------------
// Placeholder replacement
// ---------------------------------------------------------------------------

function buildReplacements(projectName) {
  const map = { project_name: projectName };
  // Pull service URLs from env vars injected by nucleo
  for (const [key, val] of Object.entries(process.env)) {
    const prefix = `${PREFIX}_`;
    if (key.startsWith(prefix) && key.endsWith("_URL")) {
      const name = key.slice(prefix.length, -4).toLowerCase().replace(/_/g, "-");
      map[`${name}_url`] = val;
    }
  }
  return map;
}

function applyReplacements(content, replacements) {
  let result = content;
  for (const [key, value] of Object.entries(replacements)) {
    result = result.replaceAll(`{{${key}}}`, value);
  }
  return result;
}

function isBinary(buffer) {
  const checkLen = Math.min(buffer.length, 8192);
  for (let i = 0; i < checkLen; i++) {
    if (buffer[i] === 0) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Scaffold
// ---------------------------------------------------------------------------

function walkDir(dir) {
  const results = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    results.push(full);
    if (entry.isDirectory()) {
      results.push(...walkDir(full));
    }
  }
  return results;
}

function copyTemplate(templateDir, destDir, replacements, dryRun) {
  const files = [];
  const entries = walkDir(templateDir);

  for (const srcPath of entries) {
    const rel = path.relative(templateDir, srcPath);
    const destPath = path.join(destDir, rel);

    if (fs.statSync(srcPath).isDirectory()) {
      if (!dryRun) fs.mkdirSync(destPath, { recursive: true });
      continue;
    }

    files.push(rel);
    if (dryRun) continue;

    const parentDir = path.dirname(destPath);
    fs.mkdirSync(parentDir, { recursive: true });

    const raw = fs.readFileSync(srcPath);
    if (isBinary(raw)) {
      fs.writeFileSync(destPath, raw);
    } else {
      const content = raw.toString("utf-8");
      const replaced = applyReplacements(content, replacements);
      fs.writeFileSync(destPath, replaced);
    }
  }

  return files;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

function cmdList() {
  const dir = resolveTemplatesDir();
  if (!dir) {
    console.error(JSON.stringify({ error: { message: "No templates directory found" } }));
    process.exit(3);
  }
  const templates = listTemplates(dir);
  console.log(JSON.stringify({ templates: templates.map((t) => ({ name: t })) }, null, 2));
}

function cmdCreate(args) {
  const dryRun = args.includes("--dry-run");
  const positional = args.filter((a) => !a.startsWith("--"));

  if (positional.length < 2) {
    console.error(
      JSON.stringify({ error: { message: "Usage: nucleo plugins new create <name> <template>" } })
    );
    process.exit(3);
  }

  const [name, templateName] = positional;
  const dir = resolveTemplatesDir();
  if (!dir) {
    console.error(JSON.stringify({ error: { message: "No templates directory found" } }));
    process.exit(3);
  }

  const available = listTemplates(dir);
  if (!available.includes(templateName)) {
    console.error(
      JSON.stringify({
        error: { message: `Template '${templateName}' not found. Available: ${available.join(", ")}` },
      })
    );
    process.exit(3);
  }

  const destDir = path.resolve(name);
  if (fs.existsSync(destDir)) {
    console.error(JSON.stringify({ error: { message: `Directory '${name}' already exists` } }));
    process.exit(3);
  }

  const templateDir = path.join(dir, templateName);
  const replacements = buildReplacements(name);
  const files = copyTemplate(templateDir, destDir, replacements, dryRun);

  console.log(
    JSON.stringify(
      {
        project: name,
        template: templateName,
        directory: destDir,
        files_count: files.length,
        dry_run: dryRun,
        files,
      },
      null,
      2
    )
  );
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

const [command, ...rest] = process.argv.slice(2);

switch (command) {
  case "list":
    cmdList();
    break;
  case "create":
    cmdCreate(rest);
    break;
  default:
    console.error(
      JSON.stringify({
        error: { message: `Unknown command: ${command}. Use 'list' or 'create'` },
      })
    );
    process.exit(3);
}
