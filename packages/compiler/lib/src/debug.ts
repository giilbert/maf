export type DebugLevel = "none" | "trace";

const LEVELS = {
  none: 0,
  trace: 1,
} satisfies Record<DebugLevel, number>;

export class DebugLogger {
  constructor(public level: DebugLevel) {
    if (level !== "none") {
      console.log(`debug: logging enabled at \`${level}\``);
    }
  }

  public trace(module: string, ...message: unknown[]) {
    if (LEVELS[this.level] < LEVELS.trace) return;
    console.log(`[trace] <${module}>`, ...message);
  }
}

export const debug = new DebugLogger("none");
