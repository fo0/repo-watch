<script lang="ts">
  import Icon from './Icon.svelte';
  import UncertainStartHelp from './UncertainStartHelp.svelte';
  import { minuteClock } from './clock';
  import { lastRunMoment } from './format';
  import { m } from './i18n/index.svelte';
  import type { LastRun } from './repoActivity';
  import { favoriteNames, type StartGate } from './startReason';
  import type { Pin } from './types';

  let {
    pins,
    isUncertain,
    repoName,
    accountName,
    gate,
    onConfigure,
    onEdit,
    onRemove,
    onLaunch,
    lastRun = () => null,
    help = false,
    demo = true,
  }: {
    pins: Pin[];
    /** The pin's target has a start with unknown outcome; its start stays locked. */
    isUncertain: (pin: Pin) => boolean;
    /** Full repository name, e.g. `owner/name`. */
    repoName: (repoId: string) => string;
    accountName: (accountId: string) => string;
    /** What a favorite's controls may do right now (`Starts.gate`). */
    gate: (pin: Pin) => StartGate;
    /** Opens the start dialog with the favorite's stored values. */
    onConfigure: (pin: Pin) => void;
    /** Opens the start dialog with the favorite section open, to edit it. */
    onEdit: (pin: Pin) => void;
    onRemove: (pinId: string) => void;
    /** Starts at once, or opens the dialog when input is missing. */
    onLaunch: (pin: Pin) => void;
    /** When the favorite's workflow last ran, on any ref, and whether it runs. */
    lastRun?: (pin: Pin) => LastRun | null;
    /** The uncertain marker opens its help (Favoriten page), not just a tooltip. */
    help?: boolean;
    demo?: boolean;
  } = $props();
</script>

<!-- One compact row per favorite, on the overview and its page alike. The
     name opens the start dialog with the stored inputs (its hit area covers
     the row), the gear edits the stored defaults, the heart removes and the
     round start at the far right starts at once. -->
<ul class="pin-rows">
  {#each pins as pin (pin.id)}
    {@const uncertain = isUncertain(pin)}
    {@const can = gate(pin)}
    {@const repo = repoName(pin.repoId)}
    {@const short = repo.split('/').pop() ?? repo}
    {@const names = favoriteNames(short, pin.label)}
    {@const last = lastRun(pin)}
    {@const moment = last && lastRunMoment(last, $minuteClock, demo)}
    <!-- `data-pin-*` lets focus move to a neighbour or a restored row; the
         name comes first, so a restored row focuses it. Name, gear, heart
         and start are off via aria-disabled: a start or a favorite change
         that locks them keeps focus. -->
    <li class="pin-row" class:uncertain data-pin-id={pin.id}>
      <!-- The time of the last run drops out whole before the repository
           shortens; „läuft“ stays. Both are in the tooltip, which is also
           the description. -->
      <button
        class="pin-open"
        onclick={() => can.canOpen && onConfigure(pin)}
        aria-label={names.open}
        title={can.openReason ??
          `${repo} · ${pin.label} · ${pin.ref} · ${accountName(pin.accountId)}${moment ? ` · ${moment.summary}` : ''}`}
        aria-disabled={can.canOpen ? undefined : 'true'}
        ><strong class="pin-repo">{short}</strong><span class="pin-rest"
          ><span class="pin-label">{pin.label}</span
          >{#if moment && !last?.active}<time
              class="pin-last"
              datetime={moment.datetime}>{moment.relative}</time
            >{/if}</span
        >{#if last?.active}<span class="pin-running" data-tone="run"
            ><Icon name="spinner" size={11} class="pin-spinner" /><Icon
              name="activity"
              size={11}
            /><span class="pin-running-word">{m().favorites.tiles.running}</span
            ></span
          >{/if}</button
      >
      <span class="branch-chip"
        ><Icon name="branch" size={11} /><span>{pin.ref}</span></span
      >
      {#if uncertain}{#if help}<UncertainStartHelp
            id={`uncertain-help-${pin.id}`}
            name={names.uncertain}
            {demo}
          />{:else}<span
            class="pin-warning"
            title={m().dispatch.uncertain.title}
            role="img"
            aria-label={names.uncertain}><Icon name="warning" size={13} /></span
          >{/if}{/if}
      <button
        class="icon-button small pin-edit"
        onclick={() => can.canOpen && onEdit(pin)}
        aria-disabled={can.canOpen ? undefined : 'true'}
        aria-label={names.edit}
        title={can.openReason ?? m().favorites.tiles.edit}
        ><Icon name="settings" size={14} /></button
      >
      <button
        class="icon-button small pin-heart"
        data-pin-remove
        aria-label={names.remove}
        title={m().favorites.tiles.remove}
        aria-disabled={can.canRemove ? undefined : 'true'}
        onclick={() => can.canRemove && onRemove(pin.id)}
        ><Icon name="heart" size={14} /></button
      >
      <!-- Starts at once when the stored request is complete; otherwise the
           start dialog opens with the favorite section to complete it. A
           held key or the second click of a double-click never starts again;
           `Favorites.launch` adds a short cooldown after a start settled. -->
      <button
        class="pin-start"
        onclick={(event) => event.detail <= 1 && can.canStart && onLaunch(pin)}
        onkeydown={(event) => {
          if (event.repeat && (event.key === 'Enter' || event.key === ' '))
            event.preventDefault();
        }}
        aria-disabled={can.canStart ? undefined : 'true'}
        aria-label={names.start}
        title={can.reason ?? m().favorites.tiles.start}
        ><Icon name="play" size={12} /></button
      >
    </li>
  {/each}
</ul>
