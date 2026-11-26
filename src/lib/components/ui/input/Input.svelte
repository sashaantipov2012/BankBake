<script lang="ts">
  import { cn } from "$lib/utils";
  import type { HTMLInputAttributes } from "svelte/elements";

  interface Props extends HTMLInputAttributes {
    value?: string | number | null;
    class?: string;
  }

  let {
    value = $bindable(""),
    class: className = "",
    ...rest
  }: Props = $props();
</script>

<input
  bind:value
  class={cn(
    "h-9 w-full rounded-sm border bg-background px-3 text-sm text-foreground transition-colors motion-reduce:transition-none placeholder:text-muted-foreground hover:border-muted-foreground/60 focus-visible:border-primary disabled:cursor-not-allowed disabled:opacity-50",
    className
  )}
  {...rest}
/>
