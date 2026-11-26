<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { AppConfig, CompilerInfo } from "$lib/types";
  import Dialog from "$lib/components/ui/dialog/Dialog.svelte";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Input from "$lib/components/ui/input/Input.svelte";
  import Switch from "$lib/components/ui/switch/Switch.svelte";
  import { AlertTriangle, CheckCircle2, RefreshCw } from "lucide-svelte";

  interface Props {
    openModal: boolean;
    config: AppConfig;
    compilerInfo: CompilerInfo;
    isDebugBuild: boolean;
    onClose: () => void;
    onSave: (cfg: AppConfig) => Promise<void>;
    onRecheckCompiler: (customPath?: string | null) => Promise<void>;
  }

  let {
    openModal,
    config,
    compilerInfo,
    isDebugBuild,
    onClose,
    onSave,
    onRecheckCompiler,
  }: Props = $props();

  let draft = $state<AppConfig>({
    mode: "standalone",
    fmodCompilerPath: null,
    outputDirectory: null,
    buildMasterBanks: true,
    exportGuids: true,
    exportProjectFile: true,
    keepTempProject: false,
    suppressUpdatePopup: false,
  });

  // Only show the suppressUpdatePopup toggle in Settings if it was already ticked on in the update popup
  let showSuppressUpdateToggle = $state(false);
  let saving = $state(false);

  // Show compiler path override if we are in a Debug/Dev build OR if the bundled FMOD files are missing
  let showCompilerOverride = $derived(
    isDebugBuild ||
      !compilerInfo.available ||
      compilerInfo.bundledAvailable === false
  );

  $effect(() => {
    if (openModal) {
      draft = {
        mode: "standalone",
        fmodCompilerPath: config.fmodCompilerPath ?? "",
        outputDirectory: config.outputDirectory ?? null,
        buildMasterBanks: config.buildMasterBanks,
        exportGuids: config.exportGuids,
        exportProjectFile: config.exportProjectFile ?? true,
        keepTempProject: config.keepTempProject,
        suppressUpdatePopup: Boolean(config.suppressUpdatePopup),
      };
      showSuppressUpdateToggle = Boolean(config.suppressUpdatePopup);
    }
  });

  async function browseFmodCompiler() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "FMOD Studio CLI",
            extensions: ["exe"],
          },
        ],
      });
      if (selected && typeof selected === "string") {
        draft.fmodCompilerPath = selected;
        await onRecheckCompiler(selected);
      }
    } catch {
      // ignore dialog cancellation
    }
  }

  async function handleSave() {
    saving = true;
    try {
      const cleaned: AppConfig = {
        mode: "standalone",
        fmodCompilerPath:
          showCompilerOverride &&
          draft.fmodCompilerPath &&
          draft.fmodCompilerPath.trim() !== ""
            ? draft.fmodCompilerPath.trim()
            : null,
        outputDirectory: config.outputDirectory,
        buildMasterBanks: draft.buildMasterBanks,
        exportGuids: draft.exportGuids,
        exportProjectFile: draft.exportProjectFile,
        keepTempProject: isDebugBuild ? draft.keepTempProject : false,
        suppressUpdatePopup: Boolean(draft.suppressUpdatePopup),
        isDebugBuild,
      };
      await onSave(cleaned);
      onClose();
    } finally {
      saving = false;
    }
  }
</script>

{#if openModal}
  <Dialog title="Settings" {onClose}>
    <section aria-labelledby="settings-output" class="space-y-3">
      <h3 id="settings-output" class="text-sm font-medium">Bake output</h3>
      <div class="divide-y rounded-md border bg-card px-3">
        <div class="setting-row">
          <div>
            <p>Include master banks</p>
            <p class="setting-help">Write the master bank and its strings file</p>
          </div>
          <Switch bind:checked={draft.buildMasterBanks} label="Include master banks" />
        </div>
        <div class="setting-row">
          <div>
            <p>Export GUIDs.txt</p>
            <p class="setting-help">Write a lookup table for event, bus, and bank IDs</p>
          </div>
          <Switch bind:checked={draft.exportGuids} label="Export GUIDs.txt" />
        </div>
        <div class="setting-row">
          <div>
            <p>Save a project archive</p>
            <p class="setting-help">Include audio and settings in a <code>.bankbake</code> file</p>
          </div>
          <Switch bind:checked={draft.exportProjectFile} label="Save a project archive" />
        </div>
        {#if showSuppressUpdateToggle}
          <div class="setting-row">
            <div>
              <p>Hide update notifications</p>
              <p class="setting-help">Turn off to show release updates on startup</p>
            </div>
            <Switch bind:checked={draft.suppressUpdatePopup} label="Hide update notifications" />
          </div>
        {/if}
      </div>
    </section>

    {#if showCompilerOverride}
      <section class="space-y-3" aria-labelledby="settings-compiler">
        <h3 id="settings-compiler" class="text-sm font-medium">FMOD compiler</h3>
        <div class="flex items-center justify-between gap-3 rounded-md border bg-card p-3">
          <div class="flex min-w-0 items-start gap-2">
            {#if compilerInfo.available}
              <CheckCircle2 class="mt-0.5 text-success" aria-hidden="true" />
            {:else}
              <AlertTriangle class="mt-0.5 text-primary" aria-hidden="true" />
            {/if}
            <div class="min-w-0 space-y-1">
              <p class="text-sm">
                {compilerInfo.available
                  ? `Compiler available (${compilerInfo.source})`
                  : "Compiler unavailable"}
              </p>
              <p class="break-all font-mono text-xs text-muted-foreground">{compilerInfo.path ?? compilerInfo.message}</p>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onclick={() => onRecheckCompiler(draft.fmodCompilerPath)}
            title="Recheck compiler"
            aria-label="Recheck compiler"
          >
            <RefreshCw aria-hidden="true" />
          </Button>
        </div>
        <div class="space-y-2">
          <label for="fmod-path" class="field-label">FMOD Studio CLI path</label>
          <div class="flex gap-2">
            <Input
              id="fmod-path"
              bind:value={draft.fmodCompilerPath}
              placeholder="Leave blank to auto-detect"
              class="min-w-0 font-mono text-xs"
            />
            <Button type="button" variant="outline" onclick={browseFmodCompiler}>Browse</Button>
          </div>
        </div>
        {#if isDebugBuild}
          <div class="setting-row">
            <div>
              <p>Keep temporary project</p>
              <p class="setting-help">Retain the generated <code>.fspro</code> and <code>bake.js</code> for debugging</p>
            </div>
            <Switch bind:checked={draft.keepTempProject} label="Keep temporary project" />
          </div>
        {/if}
      </section>
    {/if}

    {#snippet actions()}
      <Button type="button" variant="ghost" onclick={onClose}>Cancel</Button>
      <Button type="button" disabled={saving} onclick={handleSave}>
        {saving ? "Saving…" : "Save settings"}
      </Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 56px;
    padding-block: 8px;
  }

  .setting-help {
    margin-top: 4px;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }
</style>
