<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import type {
    AppConfig,
    AudioEventCardItem,
    AudioFileInfo,
    BakeBankRequest,
    BakeBankResult,
    BakeEventItemPayload,
    CompilerInfo,
    GithubReleaseInfo,
    LoadedProjectData,
  } from "$lib/types";
  import { cleanVersion, formatBytes, isNewerVersion } from "$lib/utils";
  import Button from "$lib/components/ui/button/Button.svelte";
  import Input from "$lib/components/ui/input/Input.svelte";
  import EventCard from "$lib/components/EventCard.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import UpdateModal from "$lib/components/UpdateModal.svelte";
  import {
    AlertCircle,
    AudioLines,
    CheckCircle2,
    ChevronDown,
    ChevronUp,
    Download,
    FolderOpen,
    Loader2,
    Plus,
    Settings,
  } from "lucide-svelte";

  // Display-scaled peaks from 64 equal windows of samples/amb_bird_call_01.ogg.
  const samplePeaks = [
    13, 15, 20, 15, 16, 29, 17, 14, 38, 21, 44, 40, 22, 47, 41, 74,
    52, 80, 89, 75, 48, 84, 86, 92, 89, 88, 64, 63, 99, 91, 69, 45,
    88, 100, 61, 45, 91, 84, 71, 45, 71, 82, 43, 36, 27, 22, 19, 16,
    14, 15, 14, 13, 14, 13, 13, 12, 13, 11, 11, 11, 9, 6, 5, 4,
  ];

  // Application State
  let appVersion = $state("1.1.2");
  let bankName = $state("SoundBank");
  let masterBankName = $state("Master");
  let outputDirInput = $state("./BakedBanks");
  let events = $state<AudioEventCardItem[]>([]);
  let isDragging = $state(false);
  let settingsOpen = $state(false);

  // Auto-Update Popup State
  let updateModalOpen = $state(false);
  let latestRelease = $state<GithubReleaseInfo | null>(null);

  let config = $state<AppConfig>({
    mode: "standalone",
    fmodCompilerPath: null,
    outputDirectory: null,
    buildMasterBanks: true,
    exportGuids: true,
    exportProjectFile: true,
    keepTempProject: false,
    suppressUpdatePopup: false,
    isDebugBuild: import.meta.env.DEV,
  });

  let isDebugBuild = $derived(
    config.isDebugBuild !== undefined
      ? config.isDebugBuild
      : import.meta.env.DEV
  );

  let compilerInfo = $state<CompilerInfo>({
    available: false,
    path: null,
    source: "checking",
    version: null,
    message: "Checking FMOD Studio compiler...",
  });

  let buildStatus = $state<"idle" | "baking" | "success" | "error">("idle");
  let buildError = $state<string | null>(null);
  let lastBakeResult = $state<BakeBankResult | null>(null);
  let showBuildDetails = $state(false);

  let unlistenDragDrop: (() => void) | null = null;

  async function refreshCompilerStatus(customPath?: string | null) {
    try {
      const info = await invoke<CompilerInfo>("check_fmod_compiler", {
        customPath: customPath ?? null,
      });
      compilerInfo = info;
    } catch (e: any) {
      compilerInfo = {
        available: false,
        path: null,
        source: "not_found",
        version: null,
        message: typeof e === "string" ? e : "Failed to query compiler",
      };
    }
  }

  async function loadConfig() {
    try {
      const loaded = await invoke<AppConfig>("get_app_config");
      config = {
        ...loaded,
        mode: "standalone",
      };
      outputDirInput = loaded.outputDirectory ?? "./BakedBanks";
    } catch {
      // keep defaults
    }
  }

  async function handleSaveConfig(newCfg: AppConfig) {
    const trimmedOut = outputDirInput.trim();
    const merged: AppConfig = {
      ...newCfg,
      mode: "standalone",
      outputDirectory: trimmedOut !== "" ? trimmedOut : null,
    };
    config = await invoke<AppConfig>("save_app_config", {
      newConfig: merged,
    });
    outputDirInput = config.outputDirectory ?? "./BakedBanks";
    await refreshCompilerStatus(config.fmodCompilerPath);
  }

  async function checkForGithubUpdate() {
    if (config.suppressUpdatePopup) return;
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 6000);
      const res = await fetch(
        "https://api.github.com/repos/sashaantipov2012/BankBake/releases",
        {
          headers: { Accept: "application/vnd.github+json" },
          signal: controller.signal,
        }
      );
      clearTimeout(timeoutId);
      if (!res.ok) return;

      const data = await res.json();
      const items: any[] = Array.isArray(data) ? data : data ? [data] : [];

      const newerReleases = items
        .filter((item) => !item?.draft && !item?.prerelease && item?.tag_name)
        .map((item) => {
          const rawTag: string = item.tag_name;
          const ver = cleanVersion(rawTag);
          return {
            tagName: rawTag,
            version: ver,
            name: item?.name || `BankBake ${rawTag}`,
            body: typeof item?.body === "string" ? item.body.trim() : "",
            htmlUrl:
              item?.html_url ||
              "https://github.com/sashaantipov2012/BankBake/releases",
          };
        })
        .filter((r) => isNewerVersion(r.version, appVersion))
        .sort((a, b) =>
          isNewerVersion(a.version, b.version)
            ? -1
            : isNewerVersion(b.version, a.version)
              ? 1
              : 0
        );

      if (newerReleases.length > 0) {
        const top = newerReleases[0];
        latestRelease = {
          tagName: top.tagName,
          version: top.version,
          name: top.name,
          body: top.body,
          htmlUrl: top.htmlUrl,
          missedReleases: newerReleases.map((r) => ({
            tagName: r.tagName,
            version: r.version,
            name: r.name,
            body: r.body,
          })),
        };
        updateModalOpen = true;
      }
    } catch {
      // Silently ignore any network or offline errors
    }
  }

  async function handleDismissUpdateModal(neverShowAgain: boolean) {
    updateModalOpen = false;
    if (neverShowAgain && !config.suppressUpdatePopup) {
      const updated: AppConfig = {
        ...config,
        suppressUpdatePopup: true,
      };
      config = updated;
      try {
        config = await invoke<AppConfig>("save_app_config", {
          newConfig: updated,
        });
      } catch {
        // ignore persistence error
      }
    }
  }

  async function persistOutputDirectory(dirValue: string) {
    const trimmed = dirValue.trim();
    const updated: AppConfig = {
      ...config,
      mode: "standalone",
      outputDirectory: trimmed !== "" ? trimmed : null,
    };
    config = updated;
    try {
      config = await invoke<AppConfig>("save_app_config", {
        newConfig: updated,
      });
    } catch {
      // ignore persistence failure
    }
  }

  async function pickOutputFolder() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
        defaultPath:
          outputDirInput && !outputDirInput.startsWith(".")
            ? outputDirInput
            : undefined,
      });
      if (selected && typeof selected === "string") {
        outputDirInput = selected;
        await persistOutputDirectory(selected);
      }
    } catch {
      // dialog cancelled
    }
  }

  function guessCategory(info: AudioFileInfo): "SFX" | "MUSIC" {
    const lowerName = info.filename.toLowerCase();
    const lowerPath = info.path.toLowerCase();
    if (
      lowerName.includes("music") ||
      lowerName.includes("bgm") ||
      lowerName.includes("ost") ||
      lowerPath.includes("\\music\\") ||
      lowerPath.includes("/music/") ||
      (info.durationSeconds && info.durationSeconds >= 25)
    ) {
      return "MUSIC";
    }
    return "SFX";
  }

  async function addAudioPaths(paths: string[]) {
    if (!paths || paths.length === 0) return;
    buildError = null;

    // Check if any dropped/selected path is a .bankbake project file
    const projectPath = paths.find((p) =>
      p.toLowerCase().trim().endsWith(".bankbake")
    );
    const audioPaths = paths.filter(
      (p) => !p.toLowerCase().trim().endsWith(".bankbake")
    );

    if (projectPath) {
      await loadProjectFromPath(projectPath);
    }

    if (audioPaths.length === 0) return;

    try {
      const inspected = await invoke<AudioFileInfo[]>("inspect_audio_files", {
        paths: audioPaths,
      });

      const existingPaths = new Set(events.map((e) => e.filePath.toLowerCase()));
      const existingNames = new Set(events.map((e) => e.eventName.toLowerCase()));

      const added: AudioEventCardItem[] = [];
      for (const info of inspected) {
        if (existingPaths.has(info.path.toLowerCase())) {
          continue;
        }
        existingPaths.add(info.path.toLowerCase());

        const cat = guessCategory(info);
        const prefix = cat === "MUSIC" ? "event:/music/" : "event:/sfx/";
        let evName = `${prefix}${info.suggestedEventName}`;
        let suffix = 2;
        while (existingNames.has(evName.toLowerCase())) {
          evName = `${prefix}${info.suggestedEventName}_${suffix}`;
          suffix++;
        }
        existingNames.add(evName.toLowerCase());

        const dur = info.durationSeconds ?? 0;

        added.push({
          id: crypto.randomUUID(),
          filePath: info.path,
          filename: info.filename,
          eventName: evName,
          category: cat,
          looping: cat === "MUSIC",
          volumeDb: 0,
          customLoopEnabled: false,
          loopStart: 0,
          loopEnd: dur > 0 ? Number(dur.toFixed(2)) : null,
          duration: dur,
          sizeBytes: info.sizeBytes,
          showWaveform: false,
          showAdvanced: false,
        });
      }

      if (added.length > 0) {
        events = [...events, ...added];
      }
    } catch (e: any) {
      buildStatus = "error";
      buildError = typeof e === "string" ? e : e?.message ?? "Failed to import audio files.";
    }
  }

  async function loadProjectFromPath(projectPath: string) {
    buildError = null;
    try {
      const loaded = await invoke<LoadedProjectData>("load_project_file", {
        path: projectPath,
      });
      bankName = loaded.bankName || "SoundBank";
      masterBankName = loaded.masterBankName || "Master";
      events = loaded.events.map((ev) => ({
        id: ev.id || crypto.randomUUID(),
        filePath: ev.filePath,
        filename: ev.filename,
        eventName: ev.eventName,
        category: ev.category,
        looping: ev.looping,
        volumeDb: ev.volumeDb,
        customLoopEnabled: ev.customLoopEnabled,
        loopStart: ev.loopStart,
        loopEnd: ev.loopEnd,
        duration: ev.duration,
        sizeBytes: ev.sizeBytes,
        showWaveform: false,
        showAdvanced: false,
      }));
      if (buildStatus !== "baking") {
        buildStatus = "idle";
      }
    } catch (e: any) {
      buildStatus = "error";
      buildError =
        typeof e === "string"
          ? e
          : e?.message ?? "Failed to open .bankbake project file.";
    }
  }

  async function browseOpenProject() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "BankBake Project",
            extensions: ["bankbake"],
          },
        ],
      });
      if (!selected || typeof selected !== "string") return;
      await loadProjectFromPath(selected);
    } catch {
      // dialog cancelled
    }
  }

  function buildEventPayloads(): BakeEventItemPayload[] {
    return events.map((e) => ({
      id: e.id,
      filePath: e.filePath,
      filename: e.filename,
      eventName:
        e.eventName.trim() ||
        (e.category === "MUSIC"
          ? "event:/music/unnamed_event"
          : "event:/sfx/unnamed_event"),
      category: e.category,
      looping: e.looping,
      volumeDb: Number(e.volumeDb) || 0,
      customLoopEnabled: e.customLoopEnabled,
      loopStart: Number(e.loopStart) || 0,
      loopEnd:
        e.loopEnd !== null && e.loopEnd !== undefined && Number(e.loopEnd) > 0
          ? Number(e.loopEnd)
          : null,
      duration: Number(e.duration) || 1.0,
    }));
  }

  async function handleSaveProject() {
    const trimmedBank = bankName.trim() || "SoundBank";
    const trimmedMaster = masterBankName.trim() || "Master";
    if (events.length === 0) {
      buildStatus = "error";
      buildError = "Add at least one audio event before saving a project file.";
      return;
    }
    try {
      const destinationPath = await save({
        defaultPath: `${trimmedBank}.bankbake`,
        filters: [
          {
            name: "BankBake Project",
            extensions: ["bankbake"],
          },
        ],
      });
      if (!destinationPath) return;
      buildError = null;
      await invoke<string>("save_project_file", {
        bankName: trimmedBank,
        masterBankName: trimmedMaster,
        events: buildEventPayloads(),
        destinationPath,
      });
    } catch (e: any) {
      buildStatus = "error";
      buildError =
        typeof e === "string"
          ? e
          : e?.message ?? "Failed to save .bankbake project file.";
    }
  }

  async function browseAudioFiles() {
    try {
      const selected = await open({
        multiple: true,
        directory: false,
        filters: [
          {
            name: "Audio & BankBake Projects",
            extensions: [
              "wav",
              "ogg",
              "mp3",
              "flac",
              "aif",
              "aiff",
              "wma",
              "bankbake",
            ],
          },
          {
            name: "Audio Files",
            extensions: ["wav", "ogg", "mp3", "flac", "aif", "aiff", "wma"],
          },
          {
            name: "BankBake Project",
            extensions: ["bankbake"],
          },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await addAudioPaths(paths);
    } catch {
      // dialog cancelled
    }
  }

  async function loadBundledSampleFiles() {
    buildError = null;
    try {
      const samples = await invoke<AudioFileInfo[]>("list_sample_audio_files");
      if (samples.length > 0) {
        await addAudioPaths(samples.map((s) => s.path));
      }
    } catch (e: any) {
      buildStatus = "error";
      buildError = typeof e === "string" ? e : "Could not load sample audio files.";
    }
  }

  function removeEvent(id: string) {
    events = events.filter((e) => e.id !== id);
  }

  function clearAllEvents() {
    events = [];
    if (buildStatus !== "baking") {
      buildStatus = "idle";
      buildError = null;
    }
  }

  function updateEvent(updated: AudioEventCardItem) {
    events = events.map((e) => (e.id === updated.id ? updated : e));
  }

  async function handleBakeBank() {
    if (buildStatus === "baking") return;
    const trimmedBank = bankName.trim();
    if (!trimmedBank) {
      buildStatus = "error";
      buildError = "Please enter a valid bank name.";
      return;
    }
    if (events.length === 0) {
      buildStatus = "error";
      buildError = "Please import at least one audio file before baking.";
      return;
    }

    buildStatus = "baking";
    buildError = null;
    lastBakeResult = null;

    const trimmedOutDir = outputDirInput.trim();
    if (trimmedOutDir !== (config.outputDirectory ?? "")) {
      await persistOutputDirectory(trimmedOutDir);
    }

    const trimmedMaster = masterBankName.trim() || "Master";
    const payload: BakeBankRequest = {
      bankName: trimmedBank,
      masterBankName: trimmedMaster,
      mode: "standalone",
      events: buildEventPayloads(),
      outputDirectory: trimmedOutDir !== "" ? trimmedOutDir : null,
      fmodCompilerPath: config.fmodCompilerPath,
      buildMasterBanks: config.buildMasterBanks,
      exportGuids: config.exportGuids,
      exportProjectFile: config.exportProjectFile ?? true,
    };

    try {
      const result = await invoke<BakeBankResult>("bake_bank", {
        request: payload,
      });
      lastBakeResult = result;
      buildStatus = "success";
    } catch (e: any) {
      buildStatus = "error";
      buildError = typeof e === "string" ? e : e?.message ?? "Bank compilation failed.";
    }
  }

  async function openOutputFolder() {
    const targetFile =
      lastBakeResult?.builtFiles?.[0]?.path ??
      config.outputDirectory;
    if (!targetFile) return;
    try {
      await revealItemInDir(targetFile);
    } catch {
      // fallback if reveal fails
    }
  }

  onMount(async () => {
    try {
      const v = await getVersion();
      if (v) appVersion = cleanVersion(v);
    } catch {
      appVersion = "1.1.2";
    }

    await loadConfig();
    await refreshCompilerStatus();
    checkForGithubUpdate();

    // Tauri v2 native window file drag-and-drop listener
    try {
      const webview = getCurrentWebview();
      unlistenDragDrop = await webview.onDragDropEvent((event) => {
        if (event.payload.type === "over" || event.payload.type === "enter") {
          isDragging = true;
        } else if (event.payload.type === "drop") {
          isDragging = false;
          if (event.payload.paths && event.payload.paths.length > 0) {
            addAudioPaths(event.payload.paths);
          }
        } else {
          isDragging = false;
        }
      });
    } catch {
      // Fallback when running in standard browser preview
    }
  });

  onDestroy(() => {
    if (unlistenDragDrop) {
      unlistenDragDrop();
      unlistenDragDrop = null;
    }
  });
</script>

<div class="flex min-h-screen flex-col">
  <header class="sticky top-0 z-30 border-b bg-background">
    <div class="workspace-width flex min-h-14 flex-wrap items-center justify-between gap-x-6 gap-y-2 py-2">
      <div class="flex items-center gap-3">
        <AudioLines class="text-primary" aria-hidden="true" />
        <h1 class="text-base font-semibold">BankBake</h1>
        <span class="border-l pl-3 font-mono text-xs text-muted-foreground">v{appVersion}</span>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <Button type="button" variant="ghost" size="sm" onclick={browseOpenProject}>
          <FolderOpen aria-hidden="true" />
          Open project
        </Button>
        {#if events.length > 0}
          <Button type="button" variant="ghost" size="sm" onclick={handleSaveProject}>
            <Download aria-hidden="true" />
            Save project
          </Button>
        {/if}
        <Button
          type="button"
          variant="ghost"
          size="icon"
          title="Settings"
          aria-label="Open settings"
          onclick={() => (settingsOpen = true)}
        >
          <Settings aria-hidden="true" />
        </Button>
      </div>
    </div>
  </header>

  <main class="workspace-width flex flex-1 flex-col gap-6 py-6">
    <section class="space-y-4" aria-labelledby="bank-setup-title">
      <div class="flex items-center justify-between gap-4">
        <h2 id="bank-setup-title" class="text-lg font-medium">Bank setup</h2>
        <span class="font-mono text-xs text-muted-foreground">FMOD 2.02</span>
      </div>

      <div class="bank-fields rounded-xl border bg-card p-4">
        <div class="min-w-0 space-y-2">
          <label for="bank-name-input" class="field-label">Bank name</label>
          <div class="relative">
            <Input id="bank-name-input" bind:value={bankName} placeholder="SoundBank" class="pr-16 font-mono" />
            <span class="pointer-events-none absolute inset-y-0 right-3 flex items-center font-mono text-xs text-muted-foreground" aria-hidden="true">.bank</span>
          </div>
        </div>

        {#if config.buildMasterBanks}
          <div class="min-w-0 space-y-2">
            <label for="master-bank-name-input" class="field-label">Master bank name</label>
            <div class="relative">
              <Input id="master-bank-name-input" bind:value={masterBankName} placeholder="Master" class="pr-16 font-mono" />
              <span class="pointer-events-none absolute inset-y-0 right-3 flex items-center font-mono text-xs text-muted-foreground" aria-hidden="true">.bank</span>
            </div>
          </div>
        {/if}

        <div class="min-w-0 space-y-2">
          <label for="output-folder-input" class="field-label">Output folder</label>
          <div class="flex gap-2">
            <Input
              id="output-folder-input"
              bind:value={outputDirInput}
              onblur={() => persistOutputDirectory(outputDirInput)}
              placeholder="./BakedBanks"
              class="min-w-0 font-mono"
            />
            <Button type="button" variant="outline" onclick={pickOutputFolder} aria-label="Change output folder">
              Change
            </Button>
          </div>
        </div>
      </div>
    </section>

    <section
      aria-label="Audio import dropzone"
      ondragover={(e) => {
        e.preventDefault();
        isDragging = true;
      }}
      ondragleave={() => (isDragging = false)}
      ondrop={(e) => {
        e.preventDefault();
        isDragging = false;
      }}
      class="rounded-xl border bg-card transition-colors motion-reduce:transition-none {isDragging ? 'border-primary' : ''}"
      class:empty-import={events.length === 0}
    >
      {#if events.length === 0}
        <div class="import-copy">
          <h2 class="text-2xl font-medium leading-tight">Add audio to your bank</h2>
          <p class="mt-2 text-sm text-secondary-foreground">
            Drop audio files or a <code>.bankbake</code> project here.
          </p>
          <p class="mt-2 font-mono text-xs text-muted-foreground">WAV, OGG, MP3, FLAC, AIFF, WMA</p>
          <div class="mt-6 flex flex-wrap items-center gap-2">
            <Button type="button" onclick={browseAudioFiles}>
              <Plus aria-hidden="true" />
              Select audio files
            </Button>
            <Button type="button" variant="outline" onclick={loadBundledSampleFiles}>
              Load FMOD sample audio
            </Button>
          </div>
        </div>

        <figure class="sample-waveform">
          <figcaption class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
            <span class="font-mono text-xs text-secondary-foreground">amb_bird_call_01.ogg</span>
            <span class="text-xs text-muted-foreground">Bundled sample</span>
          </figcaption>
          <div class="sample-waveform-plot" aria-hidden="true">
            {#each samplePeaks as peak}
              <span style:height={`${peak}%`}></span>
            {/each}
          </div>
          <div class="sample-waveform-time" aria-hidden="true">
            <span>0:00</span><span>0:01</span><span>0:02</span><span>0:03</span>
          </div>
        </figure>
      {:else}
        <div class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
          <p class="text-sm text-secondary-foreground">Drop more audio files anywhere in the window</p>
          <div class="flex items-center gap-2">
            <Button type="button" variant="outline" size="sm" onclick={browseAudioFiles}>
              <Plus aria-hidden="true" />
              Add files
            </Button>
            <Button type="button" variant="ghost" size="sm" onclick={clearAllEvents}>Clear all</Button>
          </div>
        </div>
      {/if}
    </section>

    {#if events.length > 0}
      <section class="space-y-3" aria-labelledby="events-title">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <div class="flex items-baseline gap-3">
            <h2 id="events-title" class="text-lg font-medium">Events</h2>
            <span class="font-mono text-xs text-muted-foreground">
              {events.filter((event) => event.category === "SFX").length} SFX ·
              {events.filter((event) => event.category === "MUSIC").length} music
            </span>
          </div>
          <span class="text-xs text-muted-foreground">Path example <code>event:/sfx/player/jump</code></span>
        </div>
        <div class="overflow-hidden rounded-xl border bg-card">
          {#each events as item (item.id)}
            <EventCard {item} onRemove={removeEvent} onUpdate={updateEvent} />
          {/each}
        </div>
      </section>
    {/if}

    {#if buildStatus === "error" && buildError}
      <section class="space-y-2 rounded-xl border bg-card p-4" role="alert">
        <h2 class="flex items-center gap-2 text-sm font-medium text-destructive">
          <AlertCircle aria-hidden="true" />
          Couldn’t bake the bank
        </h2>
        <pre class="whitespace-pre-wrap break-words font-mono text-xs text-secondary-foreground">{buildError}</pre>
      </section>
    {/if}

    {#if buildStatus === "success" && lastBakeResult}
      <section class="space-y-4 rounded-xl border bg-card p-4" aria-labelledby="bake-result-title">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="min-w-0 space-y-1">
            <h2 id="bake-result-title" class="flex items-center gap-2 text-sm font-medium">
              <CheckCircle2 class="text-success" aria-hidden="true" />
              {lastBakeResult.message}
            </h2>
            <p class="break-all font-mono text-xs text-muted-foreground">{lastBakeResult.outputDirectory}</p>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button type="button" variant="outline" size="sm" onclick={openOutputFolder}>
              <FolderOpen aria-hidden="true" />
              Open output folder
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-expanded={showBuildDetails}
              onclick={() => (showBuildDetails = !showBuildDetails)}
            >
              GUIDs and logs
              {#if showBuildDetails}<ChevronUp aria-hidden="true" />{:else}<ChevronDown aria-hidden="true" />{/if}
            </Button>
          </div>
        </div>

        <ul class="divide-y border-y font-mono text-xs">
          {#each lastBakeResult.builtFiles as file (file.filename)}
            <li class="flex justify-between gap-4 py-2">
              <span class="break-all">{file.filename}</span>
              <span class="shrink-0 text-muted-foreground">{formatBytes(file.sizeBytes)}</span>
            </li>
          {/each}
        </ul>

        {#if showBuildDetails}
          <div class="grid gap-4 {lastBakeResult.guidsContent ? 'md:grid-cols-2' : ''}">
            {#if lastBakeResult.guidsContent}
              <div class="min-w-0 space-y-2">
                <h3 class="field-label">GUIDs.txt</h3>
                <pre class="log-output">{lastBakeResult.guidsContent}</pre>
              </div>
            {/if}
            <div class="min-w-0 space-y-2">
              <h3 class="field-label">FMOD compiler output</h3>
              <pre class="log-output">{lastBakeResult.compilerStdout}</pre>
            </div>
          </div>
        {/if}
      </section>
    {/if}
  </main>

  <footer class="sticky bottom-0 z-30 border-t bg-background">
    <div class="workspace-width flex min-h-16 items-center justify-between gap-3 py-3">
      <div class="min-w-0" aria-live="polite">
        <p class="break-words text-sm text-secondary-foreground">
          {#if buildStatus === "baking"}
            Baking <code>{bankName.trim() || "SoundBank"}.bank</code>
          {:else if buildStatus === "success"}
            Baked <code>{lastBakeResult?.bankName}.bank</code>
          {:else if buildStatus === "error"}
            <span class="text-destructive">Bake failed</span>
          {:else if events.length === 0}
            No audio loaded
          {:else}
            {events.length} {events.length === 1 ? "event" : "events"} in <code>{bankName.trim() || "SoundBank"}.bank</code>
          {/if}
        </p>
        {#if isDebugBuild || !compilerInfo.available}
          <button
            type="button"
            onclick={() => (settingsOpen = true)}
            title={compilerInfo.path ?? compilerInfo.message}
            class="status-link"
          >
            <span class="status-dot {compilerInfo.available ? 'bg-success' : 'bg-primary'}" aria-hidden="true"></span>
            {#if compilerInfo.available}
              FMOD compiler ready
            {:else if compilerInfo.source === "checking"}
              Checking FMOD compiler
            {:else}
              FMOD compiler unavailable
            {/if}
          </button>
        {/if}
      </div>
      <Button
        type="button"
        size="lg"
        class="shrink-0"
        disabled={buildStatus === "baking" || events.length === 0 || !compilerInfo.available || !bankName.trim()}
        onclick={handleBakeBank}
      >
        {#if buildStatus === "baking"}
          <Loader2 class="motion-safe:animate-spin" aria-hidden="true" />
          Baking…
        {:else}
          Bake bank
        {/if}
      </Button>
    </div>
  </footer>

  <SettingsModal
    openModal={settingsOpen}
    {config}
    {compilerInfo}
    {isDebugBuild}
    onClose={() => (settingsOpen = false)}
    onSave={handleSaveConfig}
    onRecheckCompiler={refreshCompilerStatus}
  />
  <UpdateModal
    openModal={updateModalOpen}
    currentVersion={appVersion}
    release={latestRelease}
    onDismiss={handleDismissUpdateModal}
  />
</div>

<style>
  .workspace-width {
    width: 100%;
    max-width: 1440px;
    margin-inline: auto;
    padding-inline: clamp(16px, 3vw, 32px);
  }

  .bank-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 168px), 1fr));
    gap: 16px;
  }

  @media (min-width: 720px) {
    .bank-fields {
      grid-auto-flow: column;
      grid-auto-columns: minmax(152px, 1fr);
      grid-template-columns: none;
    }

    .bank-fields > :last-child {
      grid-column: span 2;
    }
  }

  .empty-import {
    display: grid;
    flex: 1;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: center;
    gap: 24px;
    min-height: 288px;
    padding: clamp(16px, 4vw, 48px);
  }

  .sample-waveform {
    min-width: 0;
    padding: 16px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--background));
  }

  .sample-waveform-plot {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    align-items: center;
    gap: clamp(1px, 0.25vw, 4px);
    height: 104px;
    margin-block: 12px 8px;
    padding-inline: 2px;
    background-image:
      linear-gradient(to bottom, transparent calc(50% - 0.5px), hsl(var(--border)) 0, hsl(var(--border)) calc(50% + 0.5px), transparent 0),
      repeating-linear-gradient(to right, hsl(var(--border)) 0 1px, transparent 1px calc(100% / 3));
    border-right: 1px solid hsl(var(--border));
  }

  .sample-waveform-plot span {
    min-height: 2px;
    border-radius: 1px;
    background: hsl(var(--primary));
  }

  .sample-waveform-time {
    display: flex;
    justify-content: space-between;
    font-family: var(--mono);
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }

  .status-link {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: hsl(var(--muted-foreground));
    font-size: 12px;
    text-align: start;
  }

  .status-link:hover {
    color: hsl(var(--foreground));
  }

  .status-dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
  }

  @media (max-width: 719px) {
    .empty-import {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
