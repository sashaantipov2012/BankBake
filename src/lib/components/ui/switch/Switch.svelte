<script lang="ts">
  import { cn } from "$lib/utils";

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    id?: string;
    label?: string;
    class?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    id,
    label,
    class: className = "",
    onchange,
  }: Props = $props();

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {id}
  {disabled}
  onclick={toggle}
  class={cn(
    "inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border transition-colors duration-150 motion-reduce:transition-none disabled:cursor-not-allowed disabled:opacity-50",
    checked
      ? "border-primary bg-primary"
      : "bg-secondary",
    className
  )}
>
  <span
    class={cn(
      "pointer-events-none block h-3.5 w-3.5 rounded-full transition-transform duration-150 motion-reduce:transition-none",
      checked
        ? "translate-x-4 bg-primary-foreground"
        : "translate-x-0.5 bg-muted-foreground"
    )}
  ></span>
</button>
