/**
 * The person's own terminal on the node that holds their key — the other way
 * to do what a portal page cannot do from another machine. These build the
 * exact commands to show for copying; nothing here runs anything.
 */

/** Quote one argument for a POSIX shell, so it is pasted as one word. */
export function shellQuote(value: string): string {
  const escaped = value.replaceAll("'", String.raw`'\''`);
  return `'${escaped}'`;
}

/** Approve a device from the terminal: `epr device approve '<link>'`. */
export function approveCommand(link: string): string {
  return `epr device approve ${shellQuote(link)}`;
}

/** Begin an identity from the terminal: `epr identity begin --name '<name>'`. */
export function beginCommand(name: string): string {
  return `epr identity begin --name ${shellQuote(name)}`;
}
