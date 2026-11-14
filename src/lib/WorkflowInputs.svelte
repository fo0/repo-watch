<script lang="ts">
  import { m } from './i18n/index.svelte';
  import type { InputSpec, InputValue, Workflow } from './types';

  let {
    workflow,
    values,
    invalid = null,
    describedBy,
    disabled,
    onChange,
  }: {
    workflow: Workflow | undefined;
    values: Record<string, InputValue>;
    /** Name of the input that blocks the start; its field is marked. */
    invalid?: string | null;
    /** ID of the message that explains the marked field. */
    describedBy?: string | undefined;
    disabled: boolean;
    onChange: (name: string, value: InputValue) => void;
  } = $props();
  const inputs = $derived(workflow?.inputs ?? []);
  /** Hint for each GitHub input type; raw type names never reach the UI. */
  const types = $derived(m().dispatch.inputs.types);

  /** Text shown in a field; numbers keep their digits, unset values stay empty. */
  const text = (input: InputSpec) => String(values[input.name] ?? '');
  /** The field's note and, while it blocks the start, the message naming it. */
  const description = (input: InputSpec) =>
    [
      input.note ? `input-note-${input.name}` : '',
      input.name === invalid ? (describedBy ?? '') : '',
    ]
      .filter(Boolean)
      .join(' ') || undefined;
  const invalidFlag = (input: InputSpec) =>
    input.name === invalid ? 'true' : undefined;
  /** An empty number field is unset, not zero. */
  const parseNumber = (value: string): InputValue =>
    value === '' ? '' : Number(value);
</script>

{#if inputs.length}<div class="input-heading">
    <span>{m().dispatch.inputs.heading}</span><span
      >{m().dispatch.inputs.count(inputs.length)}</span
    >
  </div>{/if}
{#each inputs as input (input.name)}
  {#if input.type === 'boolean'}<label class="boolean-field"
      ><span><strong>{input.label}</strong><small>{types.boolean}</small></span
      ><input
        type="checkbox"
        aria-label={input.label}
        checked={values[input.name] === true}
        onchange={(event) => onChange(input.name, event.currentTarget.checked)}
        {disabled}
      /><span class="switch" aria-hidden="true"></span></label
    >
  {:else}<label class="field"
      ><span
        >{input.label}{#if input.required}<span class="required">*</span
          >{/if}<small>{types[input.type]}</small></span
      >{#if input.type === 'choice' || input.type === 'environment'}<select
          aria-label={input.label}
          aria-required={input.required ? 'true' : undefined}
          aria-invalid={invalidFlag(input)}
          aria-describedby={description(input)}
          value={text(input)}
          onchange={(event) => onChange(input.name, event.currentTarget.value)}
          {disabled}
          >{#if !input.required}<option value=""
              >{m().dispatch.inputs.unset}</option
            >{/if}{#each input.options as option, index (index)}<option
              value={option}>{option}</option
            >{/each}</select
        >{:else if input.type === 'number'}<input
          type="number"
          aria-label={input.label}
          aria-required={input.required ? 'true' : undefined}
          aria-invalid={invalidFlag(input)}
          aria-describedby={description(input)}
          value={text(input)}
          oninput={(event) =>
            onChange(input.name, parseNumber(event.currentTarget.value))}
          step="any"
          {disabled}
        />{:else}<input
          type="text"
          aria-label={input.label}
          aria-required={input.required ? 'true' : undefined}
          aria-invalid={invalidFlag(input)}
          aria-describedby={description(input)}
          value={text(input)}
          oninput={(event) => onChange(input.name, event.currentTarget.value)}
          autocomplete="off"
          {disabled}
        />{/if}{#if input.note}<small
          class="field-help"
          id={`input-note-${input.name}`}>{input.note}</small
        >{/if}</label
    >{/if}
{/each}
