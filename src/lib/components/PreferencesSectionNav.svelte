<!-- SPDX-License-Identifier: MPL-2.0 -->
<!--
  PreferencesSectionNav — left-side navigation between preferences sections.

  Props:
    sections     — ordered list of section descriptors
    activeSection — currently active section id
    onselect     — called when user selects a section
-->
<script lang="ts">
  import * as m from '$lib/paraglide/messages';

  type Section = 'keyboard' | 'appearance' | 'terminal' | 'themes' | 'connections';

  interface SectionDescriptor {
    id: Section;
    label: () => string;
  }

  interface Props {
    sections: SectionDescriptor[];
    activeSection: Section;
    onselect: (section: Section) => void;
  }

  let { sections, activeSection, onselect }: Props = $props();
</script>

<nav
  class="w-[180px] flex-shrink-0 border-r border-(--color-border) py-2"
  aria-label={m.preferences_sections_nav()}
>
  {#each sections as section (section.id)}
    <button
      class="preferences-section-nav__item w-full text-left px-4 h-[40px] text-(--font-size-ui-base) cursor-pointer
             hover:bg-(--color-hover-bg) focus-visible:outline-2 focus-visible:outline-(--color-focus-ring)"
      class:preferences-section-nav__item--active={activeSection === section.id}
      onclick={() => onselect(section.id)}
      aria-current={activeSection === section.id ? 'page' : undefined}
    >
      {section.label()}
    </button>
  {/each}
</nav>

<style>
  .preferences-section-nav__item {
    color: var(--color-text-secondary);
    border-left: 2px solid transparent;
    transition: background-color var(--duration-fast) var(--ease-out);
  }

  .preferences-section-nav__item--active {
    color: var(--color-accent-text);
    border-left-color: var(--color-accent);
    background-color: transparent;
  }
</style>
