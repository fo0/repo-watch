import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { COMMAND_NAMES, type Commands } from './ipc';

const root = fileURLToPath(new URL('../..', import.meta.url));
const read = (path: string) => readFileSync(join(root, path), 'utf8');

/**
 * Names in `generate_handler![…]`, each by its last path segment. Comments
 * and `#[…]` attributes (e.g. a `#[cfg(…)]` entry) go first, so neither a `]`
 * nor a comma inside them ends or splits the list; the list runs up to the
 * macro's closing `])`.
 */
function registeredCommands(source: string): string[] {
  const code = source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/.*$/gm, '')
    .replace(/#\[[^\]]*\]/g, '');
  const body = /generate_handler!\[([\s\S]*?)\]\s*\)/.exec(code)?.[1];
  if (!body) throw new Error('generate_handler! not found in desktop.rs');
  return body
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((path) => path.split('::').at(-1) ?? path);
}

/** A Tauri invoke call, with or without a type argument. */
const RAW_CALL = /\binvoke\s*(?:<.*?>)?\s*\(/;
/** Any import that names invoke from the Tauri core module. */
const RAW_IMPORT =
  /import\s*(?:type\s*)?\{[^}]*\binvoke\b[^}]*\}\s*from\s*'@tauri-apps\/api\/core'/;

describe('IPC contract', () => {
  it('reads handler names past comments and attributes', () => {
    const source = `
      .invoke_handler(tauri::generate_handler![
          snapshot, // trailing note, with a comma
          /* removed_command, ] */
          #[cfg(feature = "desktop")]
          crate::desktop_auth::connect_token,
          #[cfg(any(windows, target_os = "macos"))] hide_panel
      ])
      .run(tauri::generate_context!())`;
    expect(registeredCommands(source)).toEqual([
      'snapshot',
      'connect_token',
      'hide_panel',
    ]);
  });

  it('types exactly the commands the native shell registers', () => {
    const registered = registeredCommands(read('src-tauri/src/desktop.rs'));
    expect(new Set(registered).size).toBe(registered.length);
    expect([...COMMAND_NAMES].sort()).toEqual([...registered].sort());
  });

  it('reaches the native shell only through ipc.ts', () => {
    const ipc = read('src/lib/ipc.ts');
    // The patterns must still find the one sanctioned call and import.
    expect(RAW_CALL.test(ipc) && RAW_IMPORT.test(ipc)).toBe(true);
    const offenders = readdirSync(join(root, 'src'), { recursive: true })
      .map(String)
      .filter((path) => /\.(?:ts|svelte)$/.test(path))
      .map((path) => join('src', path))
      .filter((path) => path !== join('src', 'lib', 'ipc.ts'))
      .filter((path) => {
        const source = read(path);
        return RAW_CALL.test(source) || RAW_IMPORT.test(source);
      });
    expect(offenders).toEqual([]);
  });

  /** Parameter names of one Rust handler as the camelCase keys Tauri reads. */
  function handlerArgs(command: string): string[] {
    const source = read('src-tauri/src/desktop.rs');
    const params = new RegExp(`async fn ${command}\\(([^)]*)\\)`).exec(
      source,
    )?.[1];
    return (
      (params ?? '')
        .split(',')
        .map((param) => param.split(':')[0]!.trim())
        .filter((name) => name && name !== 'app')
        // Tauri reads camelCase keys for snake_case parameters.
        .map((name) =>
          name.replace(/_(\w)/g, (_, c: string) => c.toUpperCase()),
        )
    );
  }

  it('passes set_repo_hidden the arguments its Rust handler names', () => {
    const args: Commands['set_repo_hidden']['args'] = {
      accountId: 'a',
      repoId: 'r',
      hidden: true,
    };
    expect(handlerArgs('set_repo_hidden')).toEqual(Object.keys(args));
  });

  it('passes set_owner_filter the arguments its Rust handler names', () => {
    const args: Commands['set_owner_filter']['args'] = {
      accountId: 'a',
      owners: ['o'],
    };
    expect(handlerArgs('set_owner_filter')).toEqual(Object.keys(args));
  });
});
