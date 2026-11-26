<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { AppConfig, CompilerInfo } from "$lib/types";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Input from "$lib/components/ui/input/Input.svelte";
  import Switch from "$lib/components/ui/switch/Switch.svelte";
  import {
    X,
    FolderOpen,
    Terminal,
    CheckCircle2,
    AlertTriangle,
    RefreshCw,
    Bug,
  } from "lucide-svelte";

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
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
  >
    <div
      class="w-full max-w-lg rounded-2xl border border-zinc-800 bg-zinc-900 shadow-2xl overflow-hidden"
    >
      <!-- Modal Header -->
      <div
        class="flex items-center justify-between border-b border-zinc-800 px-5 py-3.5"
      >
        <div class="flex items-center gap-2">
          <Terminal class="h-4 w-4 text-amber-400" />
          <h2 id="settings-title" class="text-sm font-semibold text-zinc-100">
            BankBake Settings
          </h2>
        </div>
        <button
          type="button"
          onclick={onClose}
          class="rounded-lg p-1 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="space-y-4 p-5 text-xs">
        <!-- Output Artifact Toggles -->
        <div class="space-y-2.5">
          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium text-zinc-200">Include Master Banks</div>
              <div class="text-[11px] text-zinc-500">
                Also output <code class="text-zinc-400">Master.bank</code> and <code class="text-zinc-400">Master.strings.bank</code>
              </div>
            </div>
            <Switch
              bind:checked={draft.buildMasterBanks}
              label="Include Master banks"
            />
          </div>

          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium text-zinc-200">Export GUIDs.txt</div>
              <div class="text-[11px] text-zinc-500">
                Export event, bus and bank GUID lookup table alongside compiled banks
              </div>
            </div>
            <Switch
              bind:checked={draft.exportGuids}
              label="Export GUIDs.txt"
            />
          </div>

          <div class="flex items-center justify-between">
            <div>
              <div class="font-medium text-zinc-200">Generate .bankbake Project File</div>
              <div class="text-[11px] text-zinc-500">
                Also save a <code class="text-zinc-400">&lt;BankName&gt;.bankbake</code> project archive with all audio assets when baking
              </div>
            </div>
            <Switch
              bind:checked={draft.exportProjectFile}
              label="Generate .bankbake project file"
            />
          </div>

          {#if showSuppressUpdateToggle}
            <div class="flex items-center justify-between pt-2 border-t border-zinc-800/70">
              <div>
                <div class="font-medium text-zinc-200">Hide Update Notifications</div>
                <div class="text-[11px] text-zinc-500">
                  Turn off to re-enable new GitHub release update popups on startup
                </div>
              </div>
              <Switch
                bind:checked={draft.suppressUpdatePopup}
                label="Hide update notifications"
              />
            </div>
          {/if}
        </div>

        <!-- Compiler Override / Debug Section (Shown in Debug Builds OR if Bundled FMOD is missing) -->
        {#if showCompilerOverride}
          <div class="space-y-3 pt-3 border-t border-amber-500/20">
            <div class="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-amber-400/90">
              {#if isDebugBuild}
                <Bug class="h-3.5 w-3.5" />
                <span>Debug Build Options</span>
              {:else}
                <AlertTriangle class="h-3.5 w-3.5" />
                <span>FMOD Studio Compiler Setup</span>
              {/if}
            </div>

            <!-- Compiler Discovery Status Banner -->
            <div
              class="flex items-start gap-2.5 rounded-xl border p-2.5 {compilerInfo.available
                ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-200'
                : 'border-amber-500/30 bg-amber-500/10 text-amber-200'}"
            >
              {#if compilerInfo.available}
                <CheckCircle2 class="h-4 w-4 shrink-0 text-emerald-400 mt-0.5" />
              {:else}
                <AlertTriangle class="h-4 w-4 shrink-0 text-amber-400 mt-0.5" />
              {/if}
              <div class="min-w-0 flex-1 space-y-0.5">
                <div class="font-medium">
                  {compilerInfo.available
                    ? `FMOD Compiler Available (${compilerInfo.source})`
                    : "FMOD Compiler Not Found"}
                </div>
                <div class="text-[11px] opacity-85 break-all font-mono">
                  {compilerInfo.path ?? compilerInfo.message}
                </div>
              </div>
              <button
                type="button"
                onclick={() => onRecheckCompiler(draft.fmodCompilerPath)}
                title="Re-verify FMOD compiler"
                class="rounded-md p-1 hover:bg-white/10 transition-colors"
              >
                <RefreshCw class="h-3.5 w-3.5" />
              </button>
            </div>

            <!-- FMOD Studio CLI Path Override -->
            <div class="space-y-1.5">
              <label for="fmod-path" class="block font-medium text-zinc-300">
                FMOD Studio CLI Path
              </label>
              <div class="flex gap-2">
                <Input
                  id="fmod-path"
                  bind:value={draft.fmodCompilerPath}
                  placeholder="Leave blank to auto-detect"
                  class="h-8 text-xs font-mono"
                />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  class="h-8 px-3 shrink-0"
                  onclick={browseFmodCompiler}
                >
                  <FolderOpen class="h-3.5 w-3.5" />
                  Browse
                </Button>
              </div>
            </div>

            <!-- Keep Temporary FMOD Project (Debug Build Only) -->
            {#if isDebugBuild}
              <div class="flex items-center justify-between">
                <div>
                  <div class="font-medium text-zinc-200">Keep Temporary FMOD Project</div>
                  <div class="text-[11px] text-zinc-500">
                    Retain generated <code class="text-zinc-400">.fspro</code> and <code class="text-zinc-400">bake.js</code> in temp directory for debugging
                  </div>
                </div>
                <Switch
                  bind:checked={draft.keepTempProject}
                  label="Keep temporary FMOD project"
                />
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Modal Footer -->
      <div
        class="flex items-center justify-end gap-2 border-t border-zinc-800 bg-zinc-950/50 px-5 py-3"
      >
        <Button type="button" variant="ghost" size="sm" onclick={onClose}>
          Cancel
        </Button>
        <Button
          type="button"
          variant="default"
          size="sm"
          disabled={saving}
          onclick={handleSave}
        >
          {saving ? "Saving..." : "Save Settings"}
        </Button>
      </div>
    </div>
  </div>
{/if}
