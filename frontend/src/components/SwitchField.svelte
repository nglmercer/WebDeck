<script lang="ts">
  /** Toggle-switch row (`.setting` + `label.switch` pattern). */
  interface Props {
    dark: string;
    containerClass: string;
    label: string;
    id: string;
    name: string;
    checked: boolean;
    extraClass?: string | undefined;
  }

  let { dark, containerClass, label, id, name, checked, extraClass }: Props = $props();
</script>

<div class="setting wd2-switchrow {containerClass}{extraClass ? ` ${extraClass}` : ''}">
  <p> {label} </p>
  <label for={id} class="switch">
    <input class={dark} type="checkbox" {id} {name} {checked} />
    <span class="slider round"></span>
  </label>
</div>
