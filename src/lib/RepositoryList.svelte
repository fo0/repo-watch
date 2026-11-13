<script lang="ts">
  import Icon from './Icon.svelte';
  import RepoInfoRow from './RepoInfoRow.svelte';
  import RepoMenu from './RepoMenu.svelte';
  import RunStrip from './RunStrip.svelte';
  import type { NoticeTone } from './StatusBanners.svelte';
  import { minuteClock } from './clock';
  import { dateTime, lastRunMoment, relativeTime } from './format';
  import { m } from './i18n/index.svelte';
  import { fullRepo } from './model';
  import { workflowTarget } from './pins';
  import { lastRun, type repositoryActivity } from './repoActivity';
  import { TONE_ICON } from './runs';
  import type { StartGate } from './startReason';
  import type { DispatchTarget, Pin, Repo, Workflow } from './types';

  let {
    groups,
    onopen,
    onfavorite,
    favorite,
    gate,
    uncertainRefs,
    expanded,
    onToggle,
    onOpen,
    opening,
    native,
    demo,
    onNotice,
    onHide,
  }: {
    groups: ReturnType<typeof repositoryActivity>;
    onopen: (workflow: Workflow) => void;
    onfavorite: (workflow: Workflow) => void;
    favorite: (workflow: Workflow) => Pin | undefined;
    /** What a target's controls may do right now (`Starts.gate`). */
    gate: (target: DispatchTarget) => StartGate;
    /** Refs with a locked start in a repository or one of its workflows. */
    uncertainRefs: (scope: {
      accountId: string;
      repoId: string;
      workflowId?: number;
    }) => string[];
    /** Expanded repository IDs; owned by the panel so they survive view switches. */
    expanded: Set<string>;
    onToggle: (repoId: string) => void;
    /** Expands a repository the search found and loads its workflows. */
    onOpen: (repoId: string) => void;
    /** Repositories whose workflows are loading. */
    opening: Set<string>;
    native: boolean;
    demo: boolean;
    onNotice: (text: string, tone: NoticeTone) => void;
    /** Hides a repository from the lists, or shows a hidden one again. */
    onHide: (repo: Repo, hidden: boolean) => void;
  } = $props();
</script>

{#snippet warning(label: string)}<span
    class="row-warning"
    role="img"
    aria-label={label}
    title={label}><Icon name="warning" size={12} /></span
  >{/snippet}

{#snippet dot()}<span class="meta-dot" aria-hidden="true">·</span>{/snippet}

{#snippet when(at: string, label: string, prefix = '')}<time
    class="repository-when"
    datetime={at}
    title={`${label}: ${dateTime(at)}${demo ? m().common.demoTime : ''}`}
    >{prefix}{relativeTime(at, $minuteClock)}</time
  >{/snippet}

<!-- A repository the search found outside the automatic ones shows its
     name and last push only; its toggle loads its workflows, then it is a
     regular row. -->
<div class="repository-list">
  {#each groups as group (group.repo.id)}
    {@const id = group.repo.id}
    {@const loading = opening.has(id)}
    {@const open = expanded.has(id) && (group.loaded || loading)}
    {@const repoLocked = uncertainRefs({
      accountId: group.repo.accountId,
      repoId: group.repo.id,
    }).length}
    {@const status = group.status}
    <section
      class="repository-card"
      class:open
      aria-label={fullRepo(group.repo)}
      data-repo-id={group.repo.id}
    >
      <!-- The strip sits inside the toggle, so its summary is part of what
           the row reads when it takes focus; the „⋯“ menu is its sibling,
           drawn in a slot before the chevron (20-repo-menu.css). -->
      <div class="repository-head">
        <button
          class="repository-toggle"
          aria-expanded={open}
          aria-controls={`actions-${group.repo.id}`}
          onclick={() => (group.loaded || loading ? onToggle(id) : onOpen(id))}
        >
          <span class="repository-copy">
            <span class="repository-title"
              ><strong class="repository-name" title={fullRepo(group.repo)}
                ><span class="repository-owner">{group.repo.owner}</span><span
                  class="repository-slash">/</span
                ><span class="repository-short">{group.repo.name}</span></strong
              >{#if repoLocked}{@render warning(
                  m().repos.list.repoUncertain,
                )}{/if}</span
            >
            <small class="repository-meta"
              >{#if !group.loaded}{#if group.repo.pushedAt}{@render when(
                    group.repo.pushedAt,
                    m().repos.lastPush,
                    m().repos.pushPrefix,
                  )}{/if}{:else}<!-- With a last run the count goes to assistive tech only: the
                     time is what the row is for, and the expanded card
                     lists the workflows. --><span
                  class="repository-count"
                  class:sr-only={!!group.last}
                  >{m().repos.list.workflows(group.workflows.length)}</span
                >{#if status}{#if !group.last}{@render dot()}{/if}<span
                    class="repository-status"
                    data-tone={status.tone}
                    title={status.tone === 'stuck'
                      ? m().runs.row.stuckHint
                      : undefined}
                    ><Icon name={TONE_ICON[status.tone]} size={12} /><span
                      >{status.label}</span
                    ></span
                  >{:else if !group.last}{@render dot()}<span
                    class="repository-status"
                    ><span>{m().repos.list.noRunsYet}</span></span
                  >{/if}{#if group.repo.permission !== 'write'}<span
                    class="repository-tag"
                    >{group.repo.permission === 'read'
                      ? m().repos.list.readOnly
                      : m().repos.list.noAccess}</span
                  >{/if}{#if group.last}<span class="repository-last"
                    ><span
                      >{@render dot()}{@render when(
                        group.last.at,
                        m().repos.list.lastRun,
                      )}</span
                    ></span
                  >{/if}{/if}</small
            >
          </span>
          {#if group.loaded}<RunStrip cells={group.recent} />{/if}
          <Icon name="chevron" size={15} class="repository-chevron" />
        </button>
        <RepoMenu
          repo={group.repo}
          latest={group.latest}
          hidden={group.hidden}
          {native}
          {demo}
          {onNotice}
          onHide={(hidden) => onHide(group.repo, hidden)}
        />
      </div>
      <div
        id={`actions-${group.repo.id}`}
        class="repository-actions"
        hidden={!open}
        aria-busy={loading || undefined}
      >
        {#if open}<RepoInfoRow
            repo={group.repo}
            {native}
            {demo}
            {onNotice}
          />{/if}
        {#if !group.loaded}<p class="repository-no-actions">
            {m().repos.list.loadingWorkflows}
          </p>{:else}{#each group.workflows as workflow (workflow.id)}
            {@const pin = favorite(workflow)}
            {@const locked = uncertainRefs({
              accountId: group.repo.accountId,
              repoId: group.repo.id,
              workflowId: workflow.id,
            })}
            {@const can = gate(workflowTarget(group.repo, workflow))}
            {@const unavailable =
              !workflow.dispatch || group.repo.permission !== 'write'}
            {@const detail = !workflow.dispatch
              ? m().repos.list.noManualStart
              : group.repo.permission !== 'write'
                ? m().repos.list.readOnly
                : group.repo.defaultBranch}
            {@const last = lastRun(group, id, workflow.id)}
            {@const moment = last && lastRunMoment(last, $minuteClock, demo)}
            <div class="repository-action">
              <button
                class="action-copy"
                onclick={() => onopen(workflow)}
                aria-label={`${workflow.name} · ${detail}${locked.length ? ` · ${m().repos.list.uncertain} (${locked.join(', ')})` : ''}`}
                title={can.openReason ??
                  `${workflow.name} · ${detail}${moment ? ` · ${moment.summary}` : ''}`}
                disabled={!can.canOpen}
              >
                <span class="action-dot"></span>
                <span
                  ><strong
                    >{workflow.name}{#if locked.length}{@render warning(
                        `${m().repos.list.uncertain} · ${locked.join(', ')}`,
                      )}{/if}</strong
                  ><small
                    >{detail}{#if last && moment}&nbsp;·&nbsp;{#if last.active}<span
                          class="tone-dot action-running"
                          data-tone="run">{m().repos.list.running}</span
                        >{:else}<time datetime={moment.datetime}
                          >{moment.relative}</time
                        >{/if}{/if}</small
                  ></span
                >
              </button>
              <button
                class="icon-button favorite-button"
                aria-pressed={!!pin}
                aria-label={pin
                  ? m().repos.list.removeFavorite(workflow.name)
                  : m().repos.list.saveFavorite(workflow.name)}
                title={unavailable
                  ? m().repos.list.favoriteUnavailable
                  : (can.openReason ??
                    (pin
                      ? m().repos.list.removeFavoriteTitle(pin.label)
                      : m().repos.list.saveFavoriteTitle))}
                disabled={unavailable}
                aria-disabled={can.canRemove ? undefined : 'true'}
                onclick={() => can.canRemove && onfavorite(workflow)}
              >
                <Icon name="heart" size={17} />
              </button>
              <button
                class="action-start"
                onclick={() => onopen(workflow)}
                aria-label={m().repos.list.startLabel(workflow.name)}
                title={can.reason ?? m().repos.list.startTitle}
                disabled={!can.canStart}
              >
                <Icon name="play" size={12} />{m().repos.list.start}
              </button>
            </div>
          {:else}<p class="repository-no-actions">
              {m().repos.list.noMatchingWorkflows}
            </p>{/each}{/if}
      </div>
    </section>
  {/each}
</div>
