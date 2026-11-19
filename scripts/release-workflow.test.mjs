import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  chmodSync,
  copyFileSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const workflow = readFileSync(
  path.join(root, '.github/workflows/desktop-release.yml'),
  'utf8',
);
const pwsh = process.env.POWERSHELL_EXECUTABLE || 'pwsh';
const available =
  spawnSync(pwsh, ['--version'], { encoding: 'utf8' }).status === 0;
if (process.env.CI && !available)
  throw new Error('PowerShell is required for release workflow tests on CI');
const options = {
  skip: available ? false : 'Install PowerShell 7 to execute workflow tests',
};

// Execute the actual Action run blocks, not copies of their release logic.
/** @param {string} job */
function scripts(job) {
  const start = workflow.indexOf(`  ${job}:`);
  assert.notEqual(start, -1);
  const rest = workflow.slice(start);
  const end = rest.slice(1).search(/^ {2}[a-z][a-z-]*:/m);
  const block = end < 0 ? rest : rest.slice(0, end + 1);
  const lines = block.split('\n');
  const result = [];
  for (let i = 0; i < lines.length; i++) {
    const match = /^(?: {6}- run: | {8}run: )(.*)$/.exec(lines[i] ?? '');
    if (!match) continue;
    if (match[1] !== '|') {
      result.push(match[1]);
      continue;
    }
    const body = [];
    while (
      i + 1 < lines.length &&
      (lines[i + 1]?.startsWith('          ') || lines[i + 1] === '')
    ) {
      body.push((lines[++i] ?? '').slice(10));
    }
    result.push(body.join('\n'));
  }
  assert.ok(result.length);
  return result;
}

// These stand-ins never contact GitHub, build an installer, or execute a downloaded EXE.
const command = String.raw`
const fs = require('node:fs');
const path = require('node:path');
const stateFile = process.env.RELEASE_TEST_STATE;
const state = JSON.parse(fs.readFileSync(stateFile, 'utf8'));
const args = process.argv.slice(2);
const after = (flag) => args[args.indexOf(flag) + 1];
const save = () => fs.writeFileSync(stateFile, JSON.stringify(state));
const fail = (message) => { console.error(message); process.exit(1); };
if (process.env.RELEASE_TEST_COMMAND === 'pnpm') {
  if (args[0] === 'tauri') {
    const config = JSON.parse(fs.readFileSync(after('--config'), 'utf8'));
    if (config.bundle.createUpdaterArtifacts !== false) fail('Unsigned build tried to require signing');
    const version = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
    const dir = 'src-tauri/target/release/bundle/nsis';
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, 'RepoWatch_' + version + '_x64-setup.exe'), state.payload);
    state.built = true;
    save();
  }
} else if (args[0] === 'api') {
  const endpoint = args.find((arg) => arg.startsWith('repos/'));
  if (endpoint.includes('/actions/runs/')) console.log('2026-11-01T00:15:00Z');
  else if (endpoint.includes('/generate-notes')) console.log(state.emptyNotes ? '' : '## Changes\n- Test release');
  else if (endpoint.includes('/releases?')) console.log('[[]]');
  else fail('Unexpected GitHub API endpoint: ' + endpoint);
} else if (args[0] === 'release' && args[1] === 'create') {
  const uploads = args.slice(3, args.indexOf('--target'));
  if (uploads.length !== 1 || !fs.existsSync(uploads[0])) fail('Expected one existing unsigned EXE');
  if (after('--target') !== process.env.RELEASE_SOURCE_SHA) fail('Wrong source SHA');
  state.created = true;
  state.uploads = uploads;
  state.release.tag_name = args[2];
  state.release.draft = false;
  save();
} else if (args[0] === 'release' && args[1] === 'download') {
  if (state.downloadFails) fail('Synthetic download failure');
  fs.mkdirSync(after('--dir'), { recursive: true });
  fs.writeFileSync(path.join(after('--dir'), after('--pattern')), state.downloadPayload ?? state.payload);
} else if (args[0] === 'release' && args[1] === 'view') {
  if (after('--json') !== 'isDraft' || !args.includes('--jq')) fail('Unexpected release view');
  console.log(String(state.release.draft));
} else fail('Unexpected GitHub command');
`;

/**
 * @param {import('node:test').TestContext} t
 * @param {Record<string, unknown>} [changes]
 */
function fixture(t, changes = {}) {
  const directory = mkdtempSync(
    path.join(tmpdir(), 'repowatch-release-action-'),
  );
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  mkdirSync(path.join(directory, 'scripts'));
  mkdirSync(path.join(directory, 'src-tauri'));
  for (const file of [
    'scripts/release.mjs',
    'scripts/updater.mjs',
    'package.json',
    'src-tauri/Cargo.toml',
    'src-tauri/Cargo.lock',
    'src-tauri/tauri.conf.json',
    'src-tauri/tauri.release.conf.json',
  ])
    copyFileSync(path.join(root, file), path.join(directory, file));
  const inheritedPath = process.env.PATH ?? '';
  /** @type {NodeJS.ProcessEnv} */
  const env = { ...process.env, GH_TOKEN: 'synthetic-test-token' };
  for (const name of Object.keys(env)) {
    if (
      name.toLowerCase() === 'path' ||
      name.startsWith('TAURI_SIGNING_') ||
      name.startsWith('REPOWATCH_UPDATER_')
    )
      delete env[name];
  }
  /** @param {string[]} args */
  const git = (...args) => {
    const result = spawnSync(
      'git',
      [
        '-c',
        'user.name=Release Test',
        '-c',
        'user.email=release-test@example.invalid',
        ...args,
      ],
      { cwd: directory, env, encoding: 'utf8' },
    );
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  git('init', '--initial-branch=main');
  git('add', '.');
  git('commit', '-m', 'Synthetic release source');
  const sha = git('rev-parse', 'HEAD');
  git('remote', 'add', 'origin', directory);
  git('update-ref', 'refs/remotes/origin/main', sha);
  const payload = 'Synthetic EXE bytes; never executable.';
  const release = { tag_name: '', draft: true };
  const stateFile = path.join(directory, 'state.json');
  const actionsEnv = path.join(directory, 'github-env');
  writeFileSync(stateFile, JSON.stringify({ payload, release, ...changes }));
  const bin = path.join(directory, 'bin');
  mkdirSync(bin);
  // cargo and rustup are no-ops: the Action must never touch a real toolchain here.
  for (const name of ['gh', 'pnpm', 'cargo', 'rustup']) {
    const mock = path.join(bin, `${name}.cjs`);
    writeFileSync(
      mock,
      `process.env.RELEASE_TEST_COMMAND = ${JSON.stringify(name)};\n${['gh', 'pnpm'].includes(name) ? command : ''}`,
    );
    if (process.platform === 'win32') {
      writeFileSync(
        path.join(bin, `${name}.cmd`),
        `@"${process.execPath}" "${mock}" %*\r\n`,
      );
    } else {
      const launcher = path.join(bin, name);
      writeFileSync(
        launcher,
        `#!/usr/bin/env node\nimport(${JSON.stringify(pathToFileURL(mock).href)});\n`,
      );
      chmodSync(launcher, 0o755);
    }
  }
  Object.assign(env, {
    PATH: `${bin}${path.delimiter}${inheritedPath}`,
    RELEASE_TEST_STATE: stateFile,
    GITHUB_REF: 'refs/heads/main',
    GITHUB_REPOSITORY: 'fo0/repo-watch',
    GITHUB_RUN_ID: '1',
    GITHUB_ENV: actionsEnv,
    RELEASE_SOURCE_SHA: sha,
  });
  /** @param {string} job */
  function run(job) {
    for (const script of scripts(job)) {
      const file = path.join(directory, 'action.ps1');
      writeFileSync(
        file,
        `$ErrorActionPreference = 'Stop'\n${script}\nif (Test-Path variable:LASTEXITCODE) { exit $LASTEXITCODE }\n`,
      );
      const result = spawnSync(pwsh, ['-NoLogo', '-NoProfile', '-File', file], {
        cwd: directory,
        env,
        encoding: 'utf8',
        timeout: 30_000,
      });
      if (result.status !== 0) return result;
      try {
        for (const line of readFileSync(actionsEnv, 'utf8').split(/\r?\n/)) {
          const equals = line.indexOf('=');
          if (equals > 0) env[line.slice(0, equals)] = line.slice(equals + 1);
        }
      } catch (error) {
        if (!(
          error instanceof Error &&
          'code' in error &&
          error.code === 'ENOENT'
        ))
          throw error;
      }
    }
    return { status: 0, stdout: '', stderr: '' };
  }
  return {
    run,
    state: () => JSON.parse(readFileSync(stateFile, 'utf8')),
  };
}

test(
  'the actual new-release Action builds and publishes an EXE with no signing variables or sidecars',
  options,
  (t) => {
    const f = fixture(t, { emptyNotes: true });
    const result = f.run('release');
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.equal(f.state().built, true);
    assert.equal(f.state().created, true);
    assert.equal(f.state().uploads.length, 1);
  },
);

for (const [name, changes, expected] of /** @type {const} */ ([
  ['fails', { downloadFails: true }, 'download verification failed'],
  [
    'differs',
    { downloadPayload: 'Tampered bytes' },
    'downloaded installer differs',
  ],
])) {
  test(
    `a published release fails when its EXE download ${name}`,
    options,
    (t) => {
      const f = fixture(t, changes);
      const result = f.run('release');
      assert.notEqual(result.status, 0);
      assert.ok(
        `${result.stderr}${result.stdout}`.includes(expected),
        result.stderr || result.stdout,
      );
    },
  );
}
