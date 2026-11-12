<script lang="ts">
  let {
    label,
    checked,
    onchange,
  }: { label: string; checked: boolean; onchange: (value: boolean) => void } = $props();
</script>

<label class="switch-field"
  ><span>{label}</span><input
    type="checkbox"
    {checked}
    onchange={(event) => onchange(event.currentTarget.checked)}
  /></label
>

<style>
  .switch-field {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 44px;
    margin: 12px 0;
    color: var(--text);
    font-size: 0.85rem;
  }
  input {
    appearance: none;
    cursor: pointer;
    position: relative;
    width: 38px;
    min-width: 38px;
    height: 22px;
    min-height: 22px;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    background: var(--canvas);
    border-radius: 20px;
  }
  input::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-muted);
  }
  input:checked {
    background: #c5c8ce;
    border-color: #c5c8ce;
  }
  input:checked::after {
    left: 19px;
    background: #24262b;
  }
  @media (pointer: coarse) {
    .switch-field {
      min-height: 48px;
    }
  }
</style>
