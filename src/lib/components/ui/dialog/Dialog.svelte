<script lang="ts">
  import type { Snippet } from "svelte";
  import { X } from "lucide-svelte";
  import Button from "$lib/components/ui/button/Button.svelte";

  interface Props {
    title: string;
    onClose: () => void;
    children: Snippet;
    actions: Snippet;
  }

  const titleId = $props.id();
  let { title, onClose, children, actions }: Props = $props();

  function showModal(dialog: HTMLDialogElement) {
    const trigger = document.activeElement;
    dialog.showModal();
    return {
      destroy() {
        dialog.close();
        if (trigger instanceof HTMLElement && trigger.isConnected) {
          trigger.focus({ preventScroll: true });
        }
      },
    };
  }
</script>

<dialog
  use:showModal
  aria-labelledby={titleId}
  class="dialog-panel"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  <header class="dialog-header">
    <h2 id={titleId} class="text-lg font-medium">{title}</h2>
    <Button type="button" variant="ghost" size="icon" aria-label={`Close ${title.toLowerCase()}`} onclick={onClose}>
      <X aria-hidden="true" />
    </Button>
  </header>
  <div class="dialog-body">{@render children()}</div>
  <footer class="dialog-footer">{@render actions()}</footer>
</dialog>
