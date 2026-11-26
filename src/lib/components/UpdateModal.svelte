<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { GithubReleaseInfo } from "$lib/types";
  import Dialog from "$lib/components/ui/dialog/Dialog.svelte";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Switch from "$lib/components/ui/switch/Switch.svelte";
  import { ExternalLink } from "lucide-svelte";

  interface Props {
    openModal: boolean;
    currentVersion: string;
    release: GithubReleaseInfo | null;
    onDismiss: (neverShowAgain: boolean) => void;
  }

  let { openModal, currentVersion, release, onDismiss }: Props = $props();

  let neverShowAgain = $state(false);

  $effect(() => {
    if (openModal) {
      neverShowAgain = false;
    }
  });

  async function handleOpenReleases() {
    const url =
      release?.htmlUrl ||
      "https://github.com/sashaantipov2012/BankBake/releases";
    try {
      await openUrl(url);
    } catch {
      window.open(url, "_blank", "noopener,noreferrer");
    }
    onDismiss(neverShowAgain);
  }
</script>

{#if openModal && release}
  <Dialog title="BankBake update available" onClose={() => onDismiss(neverShowAgain)}>
    <dl class="grid grid-cols-2 divide-x rounded-md border bg-card">
      <div class="space-y-1 p-3">
        <dt class="text-xs text-muted-foreground">Installed</dt>
        <dd class="font-mono text-sm">v{currentVersion}</dd>
      </div>
      <div class="space-y-1 p-3">
        <dt class="text-xs text-muted-foreground">Latest</dt>
        <dd class="font-mono text-sm text-primary">v{release.version}</dd>
      </div>
    </dl>

    {#if release.missedReleases && release.missedReleases.some((item) => item.body?.trim())}
      <section class="space-y-2" aria-labelledby="release-notes">
        <h3 id="release-notes" class="field-label">Release notes</h3>
        <div class="max-h-48 divide-y overflow-y-auto rounded-md border bg-card px-3">
          {#each release.missedReleases as item (item.tagName)}
            {#if item.body?.trim()}
              <div class="space-y-1 py-3">
                <h4 class="text-sm font-medium">
                  <span class="font-mono">v{item.version}</span>
                  {#if item.name && item.name !== item.tagName && item.name !== `v${item.version}`}
                    <span class="text-secondary-foreground">· {item.name}</span>
                  {/if}
                </h4>
                <p class="whitespace-pre-wrap break-words text-sm text-secondary-foreground">{item.body.trim()}</p>
              </div>
            {/if}
          {/each}
        </div>
      </section>
    {:else if release.body?.trim()}
      <section class="space-y-2" aria-labelledby="release-notes">
        <h3 id="release-notes" class="field-label">Release notes</h3>
        <p class="max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-md border bg-card p-3 text-sm text-secondary-foreground">{release.body.trim()}</p>
      </section>
    {/if}

    <div class="flex items-center justify-between gap-3">
      <span class="text-sm text-secondary-foreground">Hide update notifications</span>
      <Switch bind:checked={neverShowAgain} label="Hide update notifications" />
    </div>

    {#snippet actions()}
      <Button type="button" variant="ghost" onclick={() => onDismiss(neverShowAgain)}>Skip</Button>
      <Button type="button" onclick={handleOpenReleases}>
        Open GitHub releases
        <ExternalLink aria-hidden="true" />
      </Button>
    {/snippet}
  </Dialog>
{/if}
