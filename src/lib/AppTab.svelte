<script lang="ts">
  import HiddenReposSetting from './HiddenReposSetting.svelte';
  import LanguageSetting from './LanguageSetting.svelte';
  import LogSetting from './LogSetting.svelte';
  import RefreshSetting from './RefreshSetting.svelte';
  import StartupSettings from './StartupSettings.svelte';
  import ThemeSetting from './ThemeSetting.svelte';
  import type { AppearanceChoice } from './appearanceChoice.svelte';
  import type { Autostart } from './autostart.svelte';
  import type { HiddenChoice } from './hiddenChoice.svelte';
  import type { LogChoice } from './logChoice.svelte';
  import type { RefreshChoice } from './refresh.svelte';
  import type { Snapshot } from './types';

  // Settings → App. Every choice is staged and owned by the dialog, whose
  // „Speichern“ applies them; „Ordner öffnen“ acts at once.
  let {
    snapshot,
    native,
    appearance,
    refresh,
    hidden,
    log,
    autostart,
    onOpenFolder,
  }: {
    snapshot: Snapshot | null;
    native: boolean;
    appearance: AppearanceChoice;
    refresh: RefreshChoice;
    hidden: HiddenChoice;
    log: LogChoice;
    autostart: Autostart;
    onOpenFolder: () => void;
  } = $props();
</script>

<LanguageSetting bind:value={appearance.language} />
<ThemeSetting bind:value={appearance.theme} />
<RefreshSetting choice={refresh} />
<HiddenReposSetting {snapshot} choice={hidden} />
<LogSetting choice={log} {native} {onOpenFolder} />
<StartupSettings {native} {autostart} />
