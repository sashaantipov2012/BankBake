<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import WaveSurfer from "wavesurfer.js";
  import type { AudioEventCardItem } from "$lib/types";
  import { formatDuration, formatBytes } from "$lib/utils";
  import Input from "$lib/components/ui/input/Input.svelte";
  import Switch from "$lib/components/ui/switch/Switch.svelte";
  import Button from "$lib/components/ui/button/Button.svelte";
  import { Activity, ChevronDown, ChevronUp, Pause, Play, Square, Trash2 } from "lucide-svelte";

  interface Props {
    item: AudioEventCardItem;
    onRemove: (id: string) => void;
    onUpdate: (item: AudioEventCardItem) => void;
  }

  let { item = $bindable(), onRemove, onUpdate }: Props = $props();

  let isPlaying = $state(false);
  let currentTime = $state(0);
  let loadingAudio = $state(false);
  let audioError = $state<string | null>(null);

  let htmlAudio: HTMLAudioElement | null = null;
  let wavesurfer: WaveSurfer | null = null;
  let waveformContainer = $state<HTMLDivElement | null>(null);
  let blobUrl: string | null = null;
  let animFrameId: number | null = null;

  function mimeFromExt(filename: string): string {
    const ext = filename.split(".").pop()?.toLowerCase() ?? "";
    if (ext === "ogg") return "audio/ogg";
    if (ext === "mp3") return "audio/mpeg";
    if (ext === "flac") return "audio/flac";
    if (ext === "aif" || ext === "aiff") return "audio/aiff";
    return "audio/wav";
  }

  async function ensureBlobUrl(): Promise<string> {
    if (blobUrl) return blobUrl;
    loadingAudio = true;
    audioError = null;
    try {
      const bytes = await invoke<number[]>("read_audio_bytes", {
        path: item.filePath,
      });
      const uint8 = new Uint8Array(bytes);
      const blob = new Blob([uint8], { type: mimeFromExt(item.filename) });
      blobUrl = URL.createObjectURL(blob);
      return blobUrl;
    } catch (e: any) {
      audioError = typeof e === "string" ? e : e?.message ?? "Failed to load audio";
      throw e;
    } finally {
      loadingAudio = false;
    }
  }

  function dbToLinearGain(db: number): number {
    // Clamp between 0 and 1 for HTMLAudioElement preview
    const gain = Math.pow(10, db / 20);
    return Math.max(0, Math.min(1, gain));
  }

  async function initHtmlAudio() {
    if (htmlAudio) return htmlAudio;
    const url = await ensureBlobUrl();
    const audio = new Audio(url);
    audio.volume = dbToLinearGain(item.volumeDb);

    audio.addEventListener("loadedmetadata", () => {
      if (audio.duration && Number.isFinite(audio.duration) && audio.duration > 0) {
        item.duration = audio.duration;
        if (item.loopEnd === null || item.loopEnd > audio.duration) {
          item.loopEnd = Number(audio.duration.toFixed(2));
        }
        onUpdate(item);
      }
    });

    audio.addEventListener("ended", () => {
      if (item.looping) {
        const start = item.customLoopEnabled ? Math.max(0, item.loopStart) : 0;
        audio.currentTime = Math.min(start, audio.duration || 0);
        audio.play().catch(() => {});
      } else {
        isPlaying = false;
        currentTime = 0;
      }
    });

    htmlAudio = audio;
    return audio;
  }

  function monitorPlayback() {
    if (wavesurfer && item.showWaveform) {
      currentTime = wavesurfer.getCurrentTime();
      const dur = wavesurfer.getDuration() || item.duration;
      if (item.looping && item.customLoopEnabled && item.loopEnd && item.loopEnd > item.loopStart) {
        if (currentTime >= item.loopEnd) {
          wavesurfer.setTime(item.loopStart);
        }
      } else if (item.looping && dur > 0 && currentTime >= dur - 0.03) {
        wavesurfer.setTime(0);
      }
    } else if (htmlAudio) {
      currentTime = htmlAudio.currentTime;
      if (
        item.looping &&
        item.customLoopEnabled &&
        item.loopEnd &&
        item.loopEnd > item.loopStart &&
        currentTime >= item.loopEnd
      ) {
        htmlAudio.currentTime = item.loopStart;
      }
    }

    if (isPlaying) {
      animFrameId = requestAnimationFrame(monitorPlayback);
    }
  }

  async function togglePlay() {
    try {
      if (item.showWaveform) {
        if (!wavesurfer) {
          await initWaveSurfer();
        }
        if (wavesurfer) {
          wavesurfer.setVolume(dbToLinearGain(item.volumeDb));
          await wavesurfer.playPause();
          isPlaying = wavesurfer.isPlaying();
          if (isPlaying) {
            monitorPlayback();
          }
          return;
        }
      }

      const audio = await initHtmlAudio();
      audio.volume = dbToLinearGain(item.volumeDb);
      if (isPlaying) {
        audio.pause();
        isPlaying = false;
      } else {
        if (
          item.looping &&
          item.customLoopEnabled &&
          (audio.currentTime < item.loopStart ||
            (item.loopEnd && audio.currentTime >= item.loopEnd))
        ) {
          audio.currentTime = item.loopStart;
        }
        await audio.play();
        isPlaying = true;
        monitorPlayback();
      }
    } catch (err) {
      isPlaying = false;
    }
  }

  function stopPlayback() {
    if (wavesurfer) {
      wavesurfer.stop();
    }
    if (htmlAudio) {
      htmlAudio.pause();
      htmlAudio.currentTime = 0;
    }
    isPlaying = false;
    currentTime = 0;
    if (animFrameId) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
  }

  async function initWaveSurfer() {
    if (!waveformContainer || wavesurfer) return;
    const url = await ensureBlobUrl();
    if (!waveformContainer) return;

    const theme = getComputedStyle(waveformContainer);
    const accent = `hsl(${theme.getPropertyValue("--primary").trim()})`;
    const waveColor = `hsl(${theme.getPropertyValue("--muted-foreground").trim()} / 0.55)`;
    const ws = WaveSurfer.create({
      container: waveformContainer,
      waveColor,
      progressColor: accent,
      cursorColor: accent,
      barWidth: 2,
      barGap: 2,
      barRadius: 1,
      height: 48,
      normalize: true,
      url,
    });

    ws.on("ready", () => {
      const dur = ws.getDuration();
      if (dur && Number.isFinite(dur) && dur > 0) {
        item.duration = dur;
        if (item.loopEnd === null || item.loopEnd > dur) {
          item.loopEnd = Number(dur.toFixed(2));
        }
        onUpdate(item);
      }
    });

    ws.on("play", () => {
      isPlaying = true;
      monitorPlayback();
    });

    ws.on("pause", () => {
      isPlaying = false;
    });

    ws.on("finish", () => {
      if (item.looping) {
        const start = item.customLoopEnabled ? Math.max(0, item.loopStart) : 0;
        ws.setTime(start);
        ws.play();
      } else {
        isPlaying = false;
        currentTime = 0;
      }
    });

    ws.on("timeupdate", (t) => {
      currentTime = t;
    });

    wavesurfer = ws;
  }

  async function toggleWaveform() {
    stopPlayback();
    item.showWaveform = !item.showWaveform;
    onUpdate(item);
    if (item.showWaveform) {
      // Wait a microtask for container to mount
      setTimeout(() => {
        initWaveSurfer();
      }, 20);
    } else if (wavesurfer) {
      wavesurfer.destroy();
      wavesurfer = null;
    }
  }

  function setCategory(cat: "SFX" | "MUSIC") {
    item.category = cat;
    const trimmed = item.eventName.trim();
    if (cat === "MUSIC") {
      if (/^event:\/sfx(\/|$)/i.test(trimmed)) {
        item.eventName = trimmed.replace(/^event:\/sfx(\/|$)/i, "event:/music$1");
      } else if (/^\/?sfx(\/|$)/i.test(trimmed)) {
        item.eventName = trimmed.replace(/^(\/?)sfx(\/|$)/i, "$1music$2");
      }
    } else {
      if (/^event:\/music(\/|$)/i.test(trimmed)) {
        item.eventName = trimmed.replace(/^event:\/music(\/|$)/i, "event:/sfx$1");
      } else if (/^\/?music(\/|$)/i.test(trimmed)) {
        item.eventName = trimmed.replace(/^(\/?)music(\/|$)/i, "$1sfx$2");
      }
    }
    onUpdate(item);
  }

  function handleVolumeInput(e: Event) {
    const val = parseFloat((e.target as HTMLInputElement).value);
    item.volumeDb = Number.isFinite(val) ? val : 0;
    const gain = dbToLinearGain(item.volumeDb);
    if (htmlAudio) htmlAudio.volume = gain;
    if (wavesurfer) wavesurfer.setVolume(gain);
    onUpdate(item);
  }

  onMount(() => {
    // Probe duration quietly if not already probed by backend
    if (!item.duration || item.duration <= 0) {
      initHtmlAudio().catch(() => {});
    }
  });

  onDestroy(() => {
    stopPlayback();
    if (wavesurfer) {
      wavesurfer.destroy();
      wavesurfer = null;
    }
    if (blobUrl) {
      URL.revokeObjectURL(blobUrl);
      blobUrl = null;
    }
  });
</script>

<article class="border-b last:border-b-0" aria-label={item.filename}>
  <div class="event-row">
    <div class="event-source">
      <div class="flex items-center gap-1">
        <Button
          type="button"
          variant={isPlaying ? "secondary" : "outline"}
          size="icon"
          onclick={togglePlay}
          disabled={loadingAudio}
          title={isPlaying ? "Pause preview" : "Preview audio"}
          aria-label={isPlaying ? "Pause preview" : "Preview audio"}
        >
          {#if isPlaying}<Pause aria-hidden="true" />{:else}<Play aria-hidden="true" />{/if}
        </Button>
        {#if isPlaying || currentTime > 0}
          <Button type="button" variant="ghost" size="icon" onclick={stopPlayback} title="Stop preview" aria-label="Stop preview">
            <Square aria-hidden="true" />
          </Button>
        {/if}
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-medium" title={item.filePath}>{item.filename}</p>
        <p class="flex flex-wrap gap-x-2 font-mono text-xs text-muted-foreground">
          <span>
            {#if isPlaying || currentTime > 0}<span class="text-primary">{formatDuration(currentTime)}</span> / {/if}{formatDuration(item.duration)}
          </span>
          <span>{formatBytes(item.sizeBytes)}</span>
        </p>
      </div>
    </div>

    <Input
      bind:value={item.eventName}
      oninput={() => onUpdate(item)}
      placeholder={item.category === "MUSIC" ? "event:/music/track_name" : "event:/sfx/player/sound_name"}
      aria-label="Event path"
      class="h-8 min-w-0 font-mono text-xs"
    />

    <div class="event-actions flex flex-wrap items-center gap-2">
      <div class="category-control" role="group" aria-label="Event category">
        <button type="button" aria-pressed={item.category === "SFX"} onclick={() => setCategory("SFX")}>SFX</button>
        <button type="button" aria-pressed={item.category === "MUSIC"} onclick={() => setCategory("MUSIC")}>Music</button>
      </div>
      <div class="flex items-center gap-2 px-1">
        <span class="text-xs text-secondary-foreground">Loop</span>
        <Switch bind:checked={item.looping} label="Toggle looping" onchange={() => onUpdate(item)} />
      </div>
      <Button
        type="button"
        variant={item.showWaveform ? "secondary" : "ghost"}
        size="icon"
        onclick={toggleWaveform}
        title="Waveform"
        aria-label="Waveform"
        aria-expanded={item.showWaveform}
      >
        <Activity aria-hidden="true" />
      </Button>
      <Button
        type="button"
        variant={item.showAdvanced || item.volumeDb !== 0 || (item.looping && item.customLoopEnabled) ? "secondary" : "ghost"}
        size="sm"
        aria-expanded={item.showAdvanced}
        onclick={() => {
          item.showAdvanced = !item.showAdvanced;
          onUpdate(item);
        }}
      >
        Advanced
        {#if item.showAdvanced}<ChevronUp aria-hidden="true" />{:else}<ChevronDown aria-hidden="true" />{/if}
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        onclick={() => onRemove(item.id)}
        title="Remove event"
        aria-label="Remove event"
        class="hover:text-destructive"
      >
        <Trash2 aria-hidden="true" />
      </Button>
    </div>
  </div>

  {#if audioError}
    <p class="px-4 pb-3 text-sm text-destructive" role="alert">{audioError}</p>
  {/if}

  {#if item.showWaveform}
    <div class="border-t bg-background px-4 py-3">
      <div bind:this={waveformContainer} class="track-waveform"></div>
    </div>
  {/if}

  {#if item.showAdvanced}
    <div class="grid gap-4 border-t bg-background p-4 md:grid-cols-2">
      <div class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <label for="vol-{item.id}" class="field-label">Track volume</label>
          <div class="flex items-center gap-2">
            <span class="font-mono text-xs text-secondary-foreground">
              {item.volumeDb > 0 ? `+${item.volumeDb.toFixed(1)}` : item.volumeDb.toFixed(1)} dB
            </span>
            {#if item.volumeDb !== 0}
              <button
                type="button"
                onclick={() => {
                  item.volumeDb = 0;
                  if (htmlAudio) htmlAudio.volume = 1;
                  if (wavesurfer) wavesurfer.setVolume(1);
                  onUpdate(item);
                }}
                class="text-xs text-muted-foreground underline hover:text-foreground"
              >
                Reset
              </button>
            {/if}
          </div>
        </div>
        <div class="flex items-center gap-3">
          <span class="font-mono text-xs text-muted-foreground">−24</span>
          <input
            id="vol-{item.id}"
            type="range"
            min="-24"
            max="10"
            step="0.5"
            value={item.volumeDb}
            oninput={handleVolumeInput}
            class="min-w-0 flex-1 cursor-pointer accent-primary"
          />
          <span class="font-mono text-xs text-muted-foreground">+10</span>
        </div>
      </div>

      <div class="space-y-2">
        <div class="flex flex-wrap items-center justify-between gap-2">
          <span class="field-label">Loop points</span>
          <div class="flex items-center gap-2">
            <span class="text-xs text-muted-foreground">Custom points</span>
            <Switch
              bind:checked={item.customLoopEnabled}
              disabled={!item.looping}
              label="Enable custom loop points"
              onchange={() => {
                if (item.customLoopEnabled && item.loopEnd === null && item.duration > 0) {
                  item.loopEnd = Number(item.duration.toFixed(2));
                }
                onUpdate(item);
              }}
            />
          </div>
        </div>

        {#if !item.looping}
          <p class="text-xs text-muted-foreground">Turn on Loop to set custom points.</p>
        {:else if !item.customLoopEnabled}
          <p class="text-xs text-muted-foreground">
            Loops the full track{item.duration > 0 ? ` (${item.duration.toFixed(2)} s)` : ""}.
          </p>
        {:else}
          <div class="grid grid-cols-2 gap-2">
            <div class="min-w-0 space-y-1">
              <label for="lstart-{item.id}" class="field-label">Start (s)</label>
              <Input
                id="lstart-{item.id}"
                type="number"
                min="0"
                step="0.05"
                bind:value={item.loopStart}
                oninput={() => onUpdate(item)}
                class="h-8 font-mono text-xs"
              />
            </div>
            <div class="min-w-0 space-y-1">
              <label for="lend-{item.id}" class="field-label">End (s)</label>
              <Input
                id="lend-{item.id}"
                type="number"
                min="0.05"
                step="0.05"
                bind:value={item.loopEnd}
                placeholder={item.duration > 0 ? item.duration.toFixed(2) : "End of track"}
                oninput={() => onUpdate(item)}
                class="h-8 font-mono text-xs"
              />
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</article>

<style>
  .event-row {
    display: grid;
    grid-template-columns: minmax(180px, 0.9fr) minmax(156px, 1.2fr) auto;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
  }

  .event-source {
    display: grid;
    grid-template-columns: 68px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
  }

  .category-control {
    display: flex;
    height: 32px;
    flex-shrink: 0;
    align-items: stretch;
    padding: 2px;
    border: 1px solid hsl(var(--border));
    border-radius: 8px;
    background: hsl(var(--background));
  }

  .category-control button {
    border-radius: 4px;
    padding: 0 8px;
    font-size: 12px;
    color: hsl(var(--muted-foreground));
  }

  .category-control button:hover {
    color: hsl(var(--foreground));
  }

  .category-control button[aria-pressed="true"] {
    color: hsl(var(--foreground));
    background: hsl(var(--secondary));
  }

  .track-waveform {
    min-height: 64px;
    padding: 8px 0;
    cursor: pointer;
    background-image: linear-gradient(to bottom, transparent calc(50% - 0.5px), hsl(var(--border)) 0, hsl(var(--border)) calc(50% + 0.5px), transparent 0);
  }

  @media (max-width: 840px) {
    .event-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .event-actions {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 520px) {
    .event-row {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
