<script lang="ts">
  import { cn } from "$lib/utils";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: "default" | "secondary" | "outline" | "ghost" | "destructive";
    size?: "default" | "sm" | "lg" | "icon";
    class?: string;
  }

  let {
    variant = "default",
    size = "default",
    class: className = "",
    children,
    ...rest
  }: Props = $props();

  const variants: Record<NonNullable<Props["variant"]>, string> = {
    default:
      "bg-foreground text-background hover:bg-foreground/90",
    secondary:
      "border bg-secondary text-foreground hover:bg-secondary/70",
    outline:
      "border bg-background text-secondary-foreground hover:bg-secondary hover:text-foreground",
    ghost:
      "text-secondary-foreground hover:bg-secondary hover:text-foreground",
    destructive:
      "border text-destructive hover:bg-secondary",
  };

  const sizes: Record<NonNullable<Props["size"]>, string> = {
    default: "h-9 px-3 text-sm rounded-sm",
    sm: "h-8 px-3 text-xs rounded-sm",
    lg: "h-10 px-5 text-sm rounded-sm",
    icon: "h-8 w-8 rounded-sm p-0",
  };
</script>

<button
  class={cn(
    "inline-flex items-center justify-center gap-2 whitespace-nowrap font-medium transition-colors duration-150 motion-reduce:transition-none disabled:pointer-events-none disabled:opacity-50 select-none",
    variants[variant],
    sizes[size],
    className
  )}
  {...rest}
>
  {@render children?.()}
</button>
