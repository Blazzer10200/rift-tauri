// Stack-aware quick-start chips for the hero composer (home surface).
// Lives here (not in AssistantWelcome) because the spec attaches the chips to
// the composer's hero mode — shared so the detection logic has one home.
import { FolderTree, Search, FileText } from "lucide-svelte";

export type QuickStart = {
  icon: typeof FolderTree;
  title: string;
  prompt: string;
};

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

export function detectStack(files: readonly string[]): "fivem" | "generic" {
  for (const f of files) {
    const lower = f.toLowerCase();
    if (lower === "fxmanifest.lua" || lower.endsWith("/fxmanifest.lua")) return "fivem";
  }
  return "generic";
}

export function quickStartsFor(files: readonly string[]): QuickStart[] {
  return detectStack(files) === "fivem" ? FIVEM : GENERIC;
}
