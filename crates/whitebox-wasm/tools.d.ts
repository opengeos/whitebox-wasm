/** Result of running a tool. */
export interface ToolResult {
  /** Process exit code (0 = success). */
  exitCode: number;
  /** Captured stdout/stderr lines. */
  stdout: string[];
  /** New files the tool wrote, keyed by filename (e.g. the --output path's basename). */
  files: Record<string, Uint8Array>;
}

export interface RunToolOptions {
  /** CLI args, e.g. ["--input=/work/dem.tif", "--output=/work/out.tif", "--units=degrees"]. */
  args?: string[];
  /** Input files placed under /work, keyed by filename. */
  input?: Record<string, Uint8Array>;
}

/** Compile the WASI tool runner once. Omit `source` in browsers/bundlers; pass
 *  the wasm bytes or a URL/Response in Node. */
export function initTools(source?: URL | Response | BufferSource | string): Promise<WebAssembly.Module>;

/** List every available tool id. */
export function listTools(): Promise<string[]>;

/** Run one tool over an in-memory filesystem. */
export function runTool(tool: string, opts?: RunToolOptions): Promise<ToolResult>;
