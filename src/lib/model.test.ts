import { describe, expect, it } from 'vitest';
import fixture from '../../fixtures/demo.json';
import {
  defaultInputs,
  inputProblem,
  RequestEpoch,
  safePin,
  targetKey,
  validateDispatch,
} from './model';
import type { DispatchRequest, Pin, Snapshot, Workflow } from './types';

const snapshot = () => structuredClone(fixture) as Snapshot;
const request = (
  overrides: Partial<DispatchRequest> = {},
): DispatchRequest => ({
  accountId: 'demo-personal',
  repoId: 'demo-personal-kit',
  workflowId: 101,
  ref: 'main',
  inputs: { target: 'windows-x64', version: '0.1.0', dry_run: true },
  requestId: 'test-id',
  ...overrides,
});

describe('dispatch validation', () => {
  it('accepts exact typed inputs and provides typed defaults', () => {
    const data = snapshot();
    expect(validateDispatch(data, request())).toBeNull();
    expect(defaultInputs(data.workflows[0]!)).toEqual({
      target: 'windows-x64',
      version: '0.1.0',
      dry_run: true,
    });
  });
  it('falls back to false for booleans and an empty string otherwise without defaults', () => {
    const workflow: Workflow = {
      ...snapshot().workflows[0]!,
      inputs: (
        ['boolean', 'string', 'choice', 'number', 'environment'] as const
      ).map((type) => ({
        name: `no_default_${type}`,
        label: type,
        type,
        required: false,
        options: [],
      })),
    };
    expect(defaultInputs(workflow)).toEqual({
      no_default_boolean: false,
      no_default_string: '',
      no_default_choice: '',
      no_default_number: '',
      no_default_environment: '',
    });
  });
  it.each([
    [{ accountId: 'demo-work' }, /Konto und Repository/],
    [{ repoId: 'unknown' }, /Konto und Repository/],
    [{ workflowId: 102, inputs: {} }, /manuellen Start/],
    [{ workflowId: 201 }, /manuellen Start/],
    [{ ref: 'unlisted' }, /Branch oder Tag/],
    [{ requestId: ' ' }, /eindeutige ID/],
    [
      { inputs: { target: 'unknown', version: '1', dry_run: true } },
      /verfügbaren Wert/,
    ],
    [
      { inputs: { target: 'windows-x64', version: ' ', dry_run: true } },
      /Version ist erforderlich/,
    ],
    [
      { inputs: { target: 'windows-x64', version: '1', dry_run: 'true' } },
      /muss Ja oder Nein sein/,
    ],
    [{ inputs: { target: 'windows-x64', version: 1, dry_run: true } }, /Text/],
    [
      {
        inputs: {
          target: 'windows-x64',
          version: '1',
          dry_run: true,
          token: 'never-save',
        },
      },
      /Unbekannte/,
    ],
  ] satisfies [Partial<DispatchRequest>, RegExp][])(
    'rejects invalid target/inputs %j',
    (overrides, expected) => {
      expect(validateDispatch(snapshot(), request(overrides))).toMatch(
        expected,
      );
    },
  );
  it('requires selected repository, write rights, and dispatch availability on default branch', () => {
    const data = snapshot();
    data.repos[0]!.selected = false;
    expect(validateDispatch(data, request())).toBe('Repository nicht im Tray.');
    data.repos[0]!.selected = true;
    data.repos[0]!.permission = 'read';
    expect(validateDispatch(data, request())).toMatch(/Schreibzugriff/);
    data.repos[0]!.permission = 'write';
    data.workflows[0]!.refs = ['develop'];
    expect(validateDispatch(data, request({ ref: 'develop' }))).toMatch(
      /Standardbranch/,
    );
  });
  it('checks number and environment inputs with exact values', () => {
    const data = snapshot();
    const base = request({
      accountId: 'demo-work',
      repoId: 'demo-team-tools',
      workflowId: 201,
      inputs: { attempts: 3 },
    });
    expect(validateDispatch(data, base)).toBeNull();
    for (const value of [Infinity, NaN, '3', false])
      expect(
        validateDispatch(data, { ...base, inputs: { attempts: value } }),
      ).toMatch(/muss eine Zahl sein/);
    data.workflows[1]!.inputs = [
      {
        name: 'environment',
        label: 'Environment',
        type: 'environment',
        required: true,
        options: ['preview', 'staging'],
      },
    ];
    data.workflows[1]!.dispatch = true;
    expect(
      validateDispatch(
        data,
        request({ workflowId: 102, inputs: { environment: 'production' } }),
      ),
    ).toMatch(/verfügbaren Wert/);
    expect(
      validateDispatch(
        data,
        request({ workflowId: 102, inputs: { environment: 'staging' } }),
      ),
    ).toBeNull();
  });
});

describe('input problems', () => {
  it('names the first input that blocks a start, with the validation message', () => {
    const data = snapshot();
    const workflow = data.workflows[0]!;
    const inputs = request().inputs;
    expect(inputProblem(workflow, inputs)).toBeNull();
    expect(inputProblem(workflow, { ...inputs, version: ' ' })).toEqual({
      name: 'version',
      message: 'Version ist erforderlich.',
    });
    const problem = inputProblem(workflow, { ...inputs, target: 'linux' });
    expect(problem?.name).toBe('target');
    expect(
      validateDispatch(
        data,
        request({ inputs: { ...inputs, target: 'linux' } }),
      ),
    ).toBe(problem?.message);
  });
});

describe('safe pins', () => {
  it('whitelists persisted pin fields and blocks credential-named inputs', () => {
    const data = snapshot();
    const pin = {
      ...data.pins[0]!,
      label: '  My build  ',
      token: 'never-copy',
    };
    expect(safePin(data, pin)).toEqual({ ...data.pins[0]!, label: 'My build' });
    const workflow: Workflow = data.workflows[0]!;
    workflow.inputs.push({
      name: 'api_token',
      label: 'Token',
      type: 'string',
      required: false,
      options: [],
    });
    expect(() =>
      safePin(data, {
        ...data.pins[0]!,
        inputs: { ...data.pins[0]!.inputs, api_token: 'secret' },
      }),
    ).toThrow(/Geheime/);
    expect(() => safePin(data, { ...data.pins[0]!, label: ' ' })).toThrow(
      /Namen/,
    );
    expect(
      safePin(data, { ...data.pins[0], quickStart: 'yes' } as unknown as Pin)
        .quickStart,
    ).toBe(false);
  });
});

describe('stale UI response guard', () => {
  it('ignores responses from earlier account/repository selections and closed dialogs', async () => {
    const epochs = new RequestEpoch();
    const changes: string[] = [];
    let finish: (() => void) | undefined;
    const first = epochs.next();
    const delayed = new Promise<void>((resolve) => {
      finish = resolve;
    }).then(() => {
      if (epochs.current(first)) changes.push('stale');
    });
    const current = epochs.next();
    expect(epochs.current(current)).toBe(true);
    finish!();
    await delayed;
    expect(changes).toEqual([]);
    epochs.invalidate(); // closing a dialog or changing account invalidates its response
    expect(epochs.current(current)).toBe(false);
  });
});

describe('targetKey', () => {
  const target = {
    accountId: 'demo-personal',
    repoId: 'demo-repo',
    workflowId: 7,
    ref: 'main',
  };
  it('ignores inputs and request IDs', () => {
    const request: DispatchRequest = {
      ...target,
      inputs: { a: 1 },
      requestId: 'x',
    };
    expect(targetKey(request)).toBe(targetKey(target));
  });
  it('distinguishes every target field', () => {
    const base = targetKey(target);
    expect(targetKey({ ...target, ref: 'release' })).not.toBe(base);
    expect(targetKey({ ...target, workflowId: 8 })).not.toBe(base);
    expect(targetKey({ ...target, repoId: 'other' })).not.toBe(base);
    expect(targetKey({ ...target, accountId: 'demo-work' })).not.toBe(base);
  });
});
