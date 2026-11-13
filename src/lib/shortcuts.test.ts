import { describe, expect, it, vi } from 'vitest';
import {
  createShortcuts,
  isTypingTarget,
  sectionKey,
  shortcutFor,
  shortcutHint,
  type ShortcutActions,
} from './shortcuts';

const press = (key: string, extra: Partial<KeyboardEvent> = {}) => ({
  key,
  code: '',
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...extra,
});

describe('shortcutFor', () => {
  it('maps F5 and Ctrl+R (any case, any layout) to refresh', () => {
    expect(shortcutFor(press('F5'))).toEqual({ kind: 'refresh' });
    expect(shortcutFor(press('r', { ctrlKey: true }))).toEqual({
      kind: 'refresh',
    });
    expect(shortcutFor(press('R', { ctrlKey: true, shiftKey: true }))).toEqual({
      kind: 'refresh',
    });
    // Cyrillic layout: the physical R key still reloads a webview.
    expect(shortcutFor(press('к', { ctrlKey: true, code: 'KeyR' }))).toEqual({
      kind: 'refresh',
    });
    // Dvorak: the key labelled R is what counts, not its position.
    expect(shortcutFor(press('p', { ctrlKey: true, code: 'KeyR' }))).toBeNull();
    expect(shortcutFor(press('r'))).toBeNull();
  });
  it('maps Ctrl+1/2/3 from the digit row or keypad to the section pages', () => {
    expect(shortcutFor(press('1', { ctrlKey: true }))).toEqual({
      kind: 'section',
      page: 'repos',
    });
    expect(shortcutFor(press('&', { ctrlKey: true, code: 'Digit2' }))).toEqual({
      kind: 'section',
      page: 'runs',
    });
    expect(shortcutFor(press('3', { ctrlKey: true, code: 'Numpad3' }))).toEqual(
      { kind: 'section', page: 'favorites' },
    );
    expect(shortcutFor(press('4', { ctrlKey: true }))).toBeNull();
    expect(shortcutFor(press('1'))).toBeNull();
    expect(
      shortcutFor(press('1', { ctrlKey: true, shiftKey: true })),
    ).toBeNull();
  });
  it('keeps / (also typed with Shift) for search and Escape', () => {
    expect(shortcutFor(press('/', { shiftKey: true }))).toEqual({
      kind: 'search',
    });
    expect(shortcutFor(press('/', { ctrlKey: true }))).toBeNull();
    expect(shortcutFor(press('Escape'))).toEqual({ kind: 'escape' });
  });
  it('maps Ctrl+F (any case, any layout) to find', () => {
    expect(shortcutFor(press('f', { ctrlKey: true }))).toEqual({
      kind: 'find',
    });
    expect(shortcutFor(press('а', { ctrlKey: true, code: 'KeyF' }))).toEqual({
      kind: 'find',
    });
    expect(shortcutFor(press('f'))).toBeNull();
    expect(
      shortcutFor(press('F', { ctrlKey: true, shiftKey: true })),
    ).toBeNull();
  });
  it('maps Ctrl+, (any layout) to the settings', () => {
    expect(shortcutFor(press(',', { ctrlKey: true }))).toEqual({
      kind: 'settings',
    });
    // Cyrillic layout: the physical comma key types „б“.
    expect(shortcutFor(press('б', { ctrlKey: true, code: 'Comma' }))).toEqual({
      kind: 'settings',
    });
    // AZERTY: the comma key is elsewhere; the key itself counts.
    expect(
      shortcutFor(press(';', { ctrlKey: true, code: 'Comma' })),
    ).toBeNull();
    expect(shortcutFor(press(','))).toBeNull();
    expect(
      shortcutFor(press(';', { ctrlKey: true, shiftKey: true })),
    ).toBeNull();
  });
  it('leaves Alt and AltGr combinations to the keyboard layout', () => {
    expect(shortcutFor(press('1', { ctrlKey: true, altKey: true }))).toBeNull();
    expect(shortcutFor(press('F5', { altKey: true }))).toBeNull();
  });
});

describe('sectionKey and shortcutHint', () => {
  it('name the same digit per section as the key map', () => {
    expect(sectionKey('repos')).toBe(1);
    expect(sectionKey('runs')).toBe(2);
    expect(sectionKey('favorites')).toBe(3);
    expect(shortcutHint()).toContain(
      'Strg+1, 2, 3 – Repositories, Verlauf, Favoriten',
    );
    expect(shortcutHint()).toContain('zurück zur Übersicht');
    expect(shortcutHint()).toContain('Strg+, – Einstellungen');
    expect(shortcutHint()).toContain('/ oder Strg+F – Suche');
    expect(shortcutHint()).toContain('↓ / ↑ – nächster / vorheriger Treffer');
  });
});

describe('isTypingTarget', () => {
  it('detects text entry, pickers and editable content', () => {
    for (const tagName of ['INPUT', 'TEXTAREA', 'SELECT'])
      expect(isTypingTarget({ tagName } as unknown as EventTarget)).toBe(true);
    expect(
      isTypingTarget({
        tagName: 'DIV',
        isContentEditable: true,
      } as unknown as EventTarget),
    ).toBe(true);
    expect(
      isTypingTarget({ tagName: 'BUTTON' } as unknown as EventTarget),
    ).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });
});

describe('createShortcuts', () => {
  const setup = (blocked = false, escapes = false) => {
    const actions = {
      blocked: () => blocked,
      refresh: vi.fn(),
      toggleSection: vi.fn(),
      focusSearch: vi.fn(),
      openSettings: vi.fn(),
      escape: vi.fn(() => escapes),
    } satisfies ShortcutActions;
    return { actions, handle: createShortcuts(actions) };
  };
  const event = (
    key: string,
    extra: Partial<KeyboardEvent> = {},
    tagName = 'BODY',
  ) => {
    const value = {
      target: { tagName },
      defaultPrevented: false,
      isComposing: false,
      ...press(key, extra),
      preventDefault: vi.fn(() => {
        value.defaultPrevented = true;
      }),
    };
    return value;
  };
  const run = (
    handle: (event: KeyboardEvent) => void,
    value: ReturnType<typeof event>,
  ) => handle(value as unknown as KeyboardEvent);

  it('refreshes instead of reloading and toggles section pages', () => {
    const { actions, handle } = setup();
    const reload = event('F5');
    run(handle, reload);
    expect(reload.preventDefault).toHaveBeenCalled();
    expect(actions.refresh).toHaveBeenCalledTimes(1);
    const tab = event('2', { ctrlKey: true });
    run(handle, tab);
    expect(tab.preventDefault).toHaveBeenCalled();
    expect(actions.toggleSection).toHaveBeenCalledWith('runs');
    run(handle, event('/'));
    expect(actions.focusSearch).toHaveBeenCalledTimes(1);
  });
  it('refreshes once while F5 or Ctrl+R is held, still without reloading', () => {
    const { actions, handle } = setup();
    run(handle, event('F5'));
    for (const held of [
      event('F5', { repeat: true }),
      event('F5', { repeat: true }),
      event('r', { ctrlKey: true, repeat: true }),
    ]) {
      run(handle, held);
      expect(held.preventDefault).toHaveBeenCalled();
    }
    expect(actions.refresh).toHaveBeenCalledTimes(1);
    // A new press after releasing the key refreshes again.
    run(handle, event('F5'));
    expect(actions.refresh).toHaveBeenCalledTimes(2);
  });
  it('lets / type in a text field but runs commands from it', () => {
    const { actions, handle } = setup(false, true);
    const slash = event('/', {}, 'INPUT');
    run(handle, slash);
    expect(slash.preventDefault).not.toHaveBeenCalled();
    expect(actions.focusSearch).not.toHaveBeenCalled();
    for (const key of ['F5', 'r']) {
      const reload = event(key, { ctrlKey: key === 'r' }, 'INPUT');
      run(handle, reload);
      expect(reload.preventDefault).toHaveBeenCalled();
    }
    expect(actions.refresh).toHaveBeenCalledTimes(2);
    const tab = event('3', { ctrlKey: true }, 'INPUT');
    run(handle, tab);
    expect(tab.preventDefault).toHaveBeenCalled();
    expect(actions.toggleSection).toHaveBeenCalledWith('favorites');
    run(handle, event('1', { ctrlKey: true }, 'SELECT'));
    expect(actions.toggleSection).toHaveBeenLastCalledWith('repos');
    const settings = event(',', { ctrlKey: true }, 'INPUT');
    run(handle, settings);
    expect(settings.preventDefault).toHaveBeenCalled();
    expect(actions.openSettings).toHaveBeenCalledTimes(1);
    const find = event('f', { ctrlKey: true }, 'INPUT');
    run(handle, find);
    expect(find.preventDefault).toHaveBeenCalled();
    expect(actions.focusSearch).toHaveBeenCalledWith(true);
    const escape = event('Escape', {}, 'INPUT');
    run(handle, escape);
    expect(actions.escape).toHaveBeenCalledWith(escape.target);
    expect(escape.preventDefault).toHaveBeenCalled();
  });
  it('keeps IME composition and AltGr out of shortcuts in a text field', () => {
    const { actions, handle } = setup();
    const composing = event('F5', { isComposing: true }, 'INPUT');
    run(handle, composing);
    // The reload is still suppressed, but nothing runs mid-composition.
    expect(composing.preventDefault).toHaveBeenCalled();
    run(handle, event('2', { ctrlKey: true, isComposing: true }, 'INPUT'));
    // AltGr arrives as Ctrl+Alt and types a character.
    run(handle, event('2', { ctrlKey: true, altKey: true }, 'INPUT'));
    expect(actions.refresh).not.toHaveBeenCalled();
    expect(actions.toggleSection).not.toHaveBeenCalled();
  });
  it('leaves the keyboard to an open dialog but still blocks a reload', () => {
    const { actions, handle } = setup(true);
    const reload = event('F5');
    run(handle, reload);
    expect(reload.preventDefault).toHaveBeenCalled();
    for (const value of [
      event('/'),
      event('Escape'),
      event('3', { ctrlKey: true }),
      event(',', { ctrlKey: true }),
      event('f', { ctrlKey: true }),
    ])
      run(handle, value);
    expect(actions.refresh).not.toHaveBeenCalled();
    expect(actions.focusSearch).not.toHaveBeenCalled();
    expect(actions.escape).not.toHaveBeenCalled();
    expect(actions.toggleSection).not.toHaveBeenCalled();
    expect(actions.openSettings).not.toHaveBeenCalled();
  });
  it('skips keys another handler already took or an IME is composing', () => {
    const { actions, handle } = setup();
    const taken = event('/');
    taken.defaultPrevented = true;
    run(handle, taken);
    run(handle, event('/', { isComposing: true }));
    expect(actions.focusSearch).not.toHaveBeenCalled();
    const unhandled = event('Escape');
    run(handle, unhandled);
    expect(unhandled.preventDefault).not.toHaveBeenCalled();
  });
});
