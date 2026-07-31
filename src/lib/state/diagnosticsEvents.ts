import type { DiagEvent } from "./diagnostics.svelte";

export function mergeDiagEvents(
  current: DiagEvent[],
  incoming: DiagEvent[],
  cap: number,
): { events: DiagEvent[]; dropped: number } {
  const seen = new Set(current.map((event) => event.seq));
  const merged = current.slice();

  for (const event of incoming) {
    if (seen.has(event.seq)) continue;
    seen.add(event.seq);
    merged.push(event);
  }

  const dropped = Math.max(0, merged.length - cap);
  return {
    events: dropped ? merged.slice(dropped) : merged,
    dropped,
  };
}
