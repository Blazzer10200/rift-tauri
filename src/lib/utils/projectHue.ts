/** Stable identity hue for a project — hashed from its name so every surface
 *  (workspace card, project switcher, sidebar chip) derives the same color
 *  with no stored state. */
export function projectHue(name: string): number {
  let h = 5381;
  for (let i = 0; i < name.length; i++) h = ((h * 33) ^ name.charCodeAt(i)) >>> 0;
  return h % 360;
}
