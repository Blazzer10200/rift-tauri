const TIME_OPTS: Intl.DateTimeFormatOptions = { hour12: true, hour: "numeric", minute: "2-digit" };

function asDate(input: Date | string): Date | null {
  if (input instanceof Date) return Number.isFinite(input.getTime()) ? input : null;
  const d = new Date(input);
  return Number.isFinite(d.getTime()) ? d : null;
}

export function fmtAbsolute(input: Date | string): string {
  const d = asDate(input);
  if (!d) return String(input);
  return d.toLocaleString([], { hour12: true });
}

/**
 * Trey-readable time: "Today 1:30 PM" / "Yesterday 9:48 PM" / "Mon 9:48 PM" /
 * "5/11 9:48 PM" for older. Always single-line — avoids the column-wrap glitch
 * on the file panes' MODIFIED column.
 */
export function fmtRelative(input: Date | string, now: Date = new Date()): string {
  const d = asDate(input);
  if (!d) return String(input);

  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();

  const time = d.toLocaleTimeString([], TIME_OPTS);

  if (sameDay(d, now)) return `Today ${time}`;

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (sameDay(d, yesterday)) return `Yesterday ${time}`;

  const diffMs = now.getTime() - d.getTime();
  const sixDaysMs = 6 * 24 * 60 * 60 * 1000;
  if (diffMs > 0 && diffMs < sixDaysMs) {
    const weekday = d.toLocaleDateString([], { weekday: "short" });
    return `${weekday} ${time}`;
  }

  const datePart = `${d.getMonth() + 1}/${d.getDate()}`;
  return `${datePart} ${time}`;
}
