<script lang="ts">
  import * as Select from "$lib/components/ui/select/index.js";

  type Opt = { value: string; label: string; hint?: string; disabled?: boolean };

  let {
    value,
    options,
    onChange,
    placeholder = "Select…",
    disabled = false,
    ariaLabel,
  }: {
    value: string;
    options: Opt[];
    onChange: (v: string) => void;
    placeholder?: string;
    disabled?: boolean;
    ariaLabel?: string;
  } = $props();

  // bits-ui treats value "" as "nothing selected", but this component's
  // callers use "" for a real choice (e.g. "System default"). Map "" to a
  // sentinel at the Root boundary and back again in onValueChange.
  const EMPTY = "__rift_empty__";
  const rootValue = $derived(value === "" ? EMPTY : value);
  const selected = $derived(options.find((o) => o.value === value));
  const items = $derived(
    options.map((o) => ({ value: o.value === "" ? EMPTY : o.value, label: o.label, disabled: o.disabled }))
  );

  function handleValueChange(v: string) {
    const next = v === EMPTY ? "" : v;
    if (next !== value) onChange(next);
  }
</script>

<Select.Root type="single" value={rootValue} onValueChange={handleValueChange} {disabled} {items}>
  <Select.Trigger aria-label={ariaLabel}>
    <Select.Value class={selected ? "truncate" : "truncate text-fg-subtle"} {placeholder}>
      {#snippet children()}
        {selected?.label ?? placeholder}
      {/snippet}
    </Select.Value>
  </Select.Trigger>
  <Select.Content>
    {#each options as o (o.value)}
      <Select.Item value={o.value === "" ? EMPTY : o.value} label={o.label} disabled={o.disabled}>
        {#snippet children()}
          <span class="flex-1 truncate">{o.label}</span>
          {#if o.hint}<span class="text-fg-subtle text-xs">{o.hint}</span>{/if}
        {/snippet}
      </Select.Item>
    {/each}
  </Select.Content>
</Select.Root>
