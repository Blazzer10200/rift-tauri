// Stack-aware quick-start chips for the hero composer (home surface).
// Lives here (not in AssistantWelcome) because the spec attaches the chips to
// the composer's hero mode — shared so the detection logic has one home.
import { FolderTree, Search, FileText } from "lucide-svelte";

export type QuickStart = {
  icon: typeof FolderTree;
  title: string;
  prompt: string;
};

export type Stack = "fivem" | "node" | "rust" | "python" | "go" | "generic";

// Generic — work on any stack. Fallback when no distinctive marker is found.
const GENERIC: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the project",
    prompt: "Give me a high-level tour of this project — entry points, key folders, and what each does.",
  },
  {
    icon: Search,
    title: "Find TODOs & FIXMEs",
    prompt: "Grep for all TODO, FIXME, and HACK comments. Group them by file and summarize what needs attention.",
  },
  {
    icon: FileText,
    title: "Explain a key file",
    prompt: "Pick the most important file in this project and walk me through what it does.",
  },
];

// FiveM/RedM-aware — surfaced when `fxmanifest.lua` is detected anywhere.
const FIVEM: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the event surface",
    prompt: "Scan every resource under this workspace. List all RegisterNetEvent / AddEventHandler / on() handlers grouped by resource, with file:line.",
  },
  {
    icon: Search,
    title: "Find missing dependencies",
    prompt: "Read every fxmanifest.lua. For each resource's declared dependencies, verify the dependency resource exists in this workspace. List any unresolved dependencies.",
  },
  {
    icon: FileText,
    title: "Explain the boot order",
    prompt: "Walk through how this server boots: server.cfg start order if present, fxmanifest dependencies, and any explicit ensure() calls. Diagram the load chain.",
  },
];

// Node / JS-TS — surfaced when `package.json` is present.
const NODE: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the project",
    prompt: "Read package.json — list the scripts, dependencies, and dev-dependencies. Tell me the framework, the entry point, and how to run and build this project.",
  },
  {
    icon: Search,
    title: "Audit the dependencies",
    prompt: "Look at package.json's dependencies. Flag anything outdated, unused, or duplicated, and call out any that are notably heavy or risky.",
  },
  {
    icon: FileText,
    title: "Trace the entry point",
    prompt: "Find this project's entry point (main / module / the framework's convention) and walk me through what happens from startup to the first rendered output or served request.",
  },
];

// Rust — surfaced when `Cargo.toml` is present.
const RUST: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the crate",
    prompt: "Read Cargo.toml — list the crate(s), features, and dependencies. Tell me whether this is a binary or library, the entry point, and how to build, run, and test it.",
  },
  {
    icon: Search,
    title: "Find unwrap & panic risks",
    prompt: "Grep for .unwrap(), .expect(), and panic! across the crate. Group by file:line and flag the ones in non-test code that could realistically fire at runtime.",
  },
  {
    icon: FileText,
    title: "Map the module tree",
    prompt: "Starting from lib.rs or main.rs, walk the mod declarations and lay out the module tree. Explain what each top-level module is responsible for.",
  },
];

// Python — surfaced when pyproject.toml / requirements.txt / setup.py is present.
const PYTHON: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the project",
    prompt: "Read pyproject.toml / requirements.txt / setup.py — list the dependencies, the Python version, and any console-script entry points. Tell me how to install and run this project.",
  },
  {
    icon: Search,
    title: "Find the entry points",
    prompt: "Find every `if __name__ == \"__main__\"` block and any declared console-script / CLI entry points. List them with file:line and a one-line description of each.",
  },
  {
    icon: FileText,
    title: "Explain the package layout",
    prompt: "Walk the package structure — top-level packages, their __init__.py exports, and how the modules depend on each other. Point out the core module to start reading from.",
  },
];

// Go — surfaced when `go.mod` is present.
const GO: QuickStart[] = [
  {
    icon: FolderTree,
    title: "Map the module",
    prompt: "Read go.mod — give me the module path, Go version, and dependencies. List the packages under this module and which ones contain a `func main`.",
  },
  {
    icon: Search,
    title: "Find error-handling gaps",
    prompt: "Scan for ignored errors (`_ =` on error returns), unchecked type assertions, and naked panics. Group by file:line and flag the ones that matter.",
  },
  {
    icon: FileText,
    title: "Trace the entry point",
    prompt: "Find the package main / func main entry point and walk me through what the program does from startup to its main work loop or first served request.",
  },
];

const BY_STACK: Record<Stack, QuickStart[]> = {
  fivem: FIVEM,
  node: NODE,
  rust: RUST,
  python: PYTHON,
  go: GO,
  generic: GENERIC,
};

// True when `marker` sits at the workspace root or one level into any subdir.
// Paths from assistant_list_workspace_files are root-relative, forward-slash.
function hasMarker(files: readonly string[], marker: string): boolean {
  for (const f of files) {
    const lower = f.toLowerCase();
    if (lower === marker || lower.endsWith(`/${marker}`)) return true;
  }
  return false;
}

// Order matters: FiveM (most specific) wins over the Lua/Node overlap, then
// the single-language markers. Tauri/Electron projects carry both package.json
// and Cargo.toml — Node wins there because the day-to-day surface is JS/TS.
export function detectStack(files: readonly string[]): Stack {
  if (hasMarker(files, "fxmanifest.lua")) return "fivem";
  if (hasMarker(files, "package.json")) return "node";
  if (hasMarker(files, "cargo.toml")) return "rust";
  if (
    hasMarker(files, "pyproject.toml") ||
    hasMarker(files, "requirements.txt") ||
    hasMarker(files, "setup.py")
  ) {
    return "python";
  }
  if (hasMarker(files, "go.mod")) return "go";
  return "generic";
}

export function quickStartsFor(files: readonly string[]): QuickStart[] {
  return BY_STACK[detectStack(files)];
}
