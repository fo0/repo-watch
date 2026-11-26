<script lang="ts">
  import HiddenReposSetting from './HiddenReposSetting.svelte';
  import LanguageSetting from './LanguageSetting.svelte';
  import LogSetting from './LogSetting.svelte';
  import OwnerFilterSetting from './OwnerFilterSetting.svelte';
  import RefreshSetting from './RefreshSetting.svelte';
  import StartupSettings from './StartupSettings.svelte';
  import ThemeSetting from './ThemeSetting.svelte';
  import type { AppearanceChoice } from './appearanceChoice.svelte';
  import type { Autostart } from './autostart.svelte';
  import type { HiddenChoice } from './hiddenChoice.svelte';
  import type { LogChoice } from './logChoice.svelte';
  import type { OwnerChoice } from './ownerChoice.svelte';
  import type { RefreshChoice } from './refresh.svelte';
  import type { Snapshot } from './types';

  // Settings → App. Every choice is staged and owned by the dialog, whose
  // „Speichern“ applies them; „Ordner öffnen“ acts at once.
  let {
    snapshot,
    native,
    appearance,
    refresh,
    owners,
    hidden,
    log,
    autostart,
    onOpenFolder,
  }: {
    snapshot: Snapshot | null;
    native: boolean;
    appearance: AppearanceChoice;
    refresh: RefreshChoice;
    owners: OwnerChoice;
    hidden: HiddenChoice;
    log: LogChoice;
    autostart: Autostart;
    onOpenFolder: () => void;
  } = $props();
</script>

<LanguageSetting bind:value={appearance.language} />
<ThemeSetting bind:value={appearance.theme} />
<RefreshSetting choice={refresh} />
<OwnerFilterSetting {snapshot} choice={owners} />
<HiddenReposSetting {snapshot} choice={hidden} />
<LogSetting choice={log} {native} {onOpenFolder} />
<StartupSettings {native} {autostart} />
