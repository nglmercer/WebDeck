// Port of static/js/themes-setting.js. Runs after render.

function toggleDisplay(): void {
  const chooseBackgroundElement = document.getElementById('choose-themes');
  const configContainer = document.getElementById('config-container');
  if (!chooseBackgroundElement || !configContainer) return;

  if (chooseBackgroundElement.style.display === 'none') {
    chooseBackgroundElement.style.display = 'block';
    configContainer.style.display = 'none';
  } else {
    chooseBackgroundElement.style.display = 'none';
    configContainer.style.display = 'block';
  }
}

function getThemesArray(): string[] {
  const themesArrayEl = document.getElementById('choose-themes-handler') as HTMLInputElement | null;
  const themesArrayString = (themesArrayEl?.value ?? '[]').replace(/'/g, '"');
  return JSON.parse(themesArrayString) as string[];
}

function updateThemesInputValue(themesArray: string[]): void {
  const modifiedArray = themesArray.toString().replace(/,/g, "','");
  document.getElementById('choose-themes-handler')?.setAttribute('value', `['${modifiedArray}']`);
}

function swapElements(list: string[], firstElement: string, secondElement: string): string[] | null {
  const firstIndex = list.indexOf(firstElement);
  const secondIndex = list.indexOf(secondElement);

  if (firstIndex !== -1 && secondIndex !== -1) {
    const temp = list[firstIndex] as string;
    list[firstIndex] = list[secondIndex] as string;
    list[secondIndex] = temp;
    return list;
  } else {
    console.log('One or both elements are not found in the list.');
    return null;
  }
}

function swapDomElements(element1: Element, element2: Element): void {
  const parent = element1.parentNode;
  if (!parent) return;
  const index1 = Array.prototype.indexOf.call(parent.children, element1) as number;
  const index2 = Array.prototype.indexOf.call(parent.children, element2) as number;
  parent.insertBefore(element1, parent.children[index2] ?? null);
  parent.insertBefore(element2, parent.children[index1] ?? null);

  const themesArray = getThemesArray();
  swapElements(themesArray, element1.getAttribute('filename') ?? '', element2.getAttribute('filename') ?? '');
  updateThemesInputValue(themesArray);
}

function handleArrowClick(this: Element, event: Event): void {
  const target = event.target as Element;
  const themeContainer = this.closest('.theme-container');
  const parentContainer = themeContainer?.parentNode as Element | null;
  if (!themeContainer || !parentContainer) return;

  if (target.classList.contains('arrow-up-hitbox')) {
    const prevSibling = themeContainer.previousElementSibling;
    if (prevSibling !== null && !prevSibling.hasAttribute('defaulttheme')) {
      swapDomElements(themeContainer, prevSibling);
    }
  } else if (target.classList.contains('arrow-down-hitbox')) {
    const nextSibling = themeContainer.nextElementSibling;
    if (nextSibling !== null && !nextSibling.hasAttribute('defaulttheme')) {
      swapDomElements(nextSibling, themeContainer);
    }
  }

  const firstTheme = parentContainer.firstElementChild;
  const lastTheme = parentContainer.lastElementChild;
  const arrows = themeContainer.querySelectorAll('.arrow-up-hitbox, .arrow-down-hitbox');
  arrows.forEach((arrow) => {
    if (
      (themeContainer === firstTheme || firstTheme?.hasAttribute('defaulttheme')) &&
      arrow.classList.contains('arrow-up-hitbox')
    ) {
      arrow.classList.add('disabled');
    } else {
      arrow.classList.remove('disabled');
    }
    if (
      (themeContainer === lastTheme || lastTheme?.hasAttribute('defaulttheme')) &&
      arrow.classList.contains('arrow-down-hitbox')
    ) {
      arrow.classList.add('disabled');
    } else {
      arrow.classList.remove('disabled');
    }
  });
}

export function initThemesSetting(): void {
  document.getElementById('setting-themes')?.addEventListener('click', toggleDisplay);
  document.getElementById('setting-themes-back')?.addEventListener('click', toggleDisplay);

  const containers = document.querySelectorAll('.theme-container');

  containers.forEach((container) => {
    let moveArrow: Element | null = container.querySelector('.disable-theme');
    const upDownArrows = container.querySelector('.arrows-container');
    if (moveArrow === null) {
      moveArrow = container.querySelector('.enable-theme');
    }

    if (moveArrow !== null) {
      const arrow = moveArrow;
      container.addEventListener('mouseenter', function () {
        arrow.classList.remove('invisible');
      });

      container.addEventListener('mouseleave', function () {
        arrow.classList.add('invisible');
      });
    }

    if (upDownArrows !== null) {
      const arrows = upDownArrows;
      container.addEventListener('mouseenter', function () {
        const parentDiv = document.getElementById('disabled-themes');
        const isChild = parentDiv?.contains(arrows) ?? false;
        if (!isChild) {
          arrows.classList.remove('invisible');
        }
      });

      container.addEventListener('mouseleave', function () {
        arrows.classList.add('invisible');
      });
    }
  });

  document.querySelectorAll('.disable-theme-hitbox, .enable-theme-hitbox').forEach((hitbox) => {
    hitbox.addEventListener('click', function (this: Element) {
      const themeContainer = this.closest('.theme-container');
      let arrow = themeContainer?.querySelector('.disable-theme') ?? null;
      const upDownArrows = themeContainer?.querySelector('.arrows-container');
      if (arrow === null) {
        arrow = themeContainer?.querySelector('.enable-theme') ?? null;
      }

      console.log(arrow);
      console.log('Clicked:', this.classList.contains('disable-theme-hitbox') ? 'Disable Theme' : 'Enable Theme');
      console.log('Parent .theme-container:', themeContainer);

      const themePath = themeContainer?.getAttribute('filename') ?? '';
      const themesArray = getThemesArray();
      let newElement: string | undefined;

      console.log(themesArray);

      for (let i = 0; i < themesArray.length; i++) {
        if ((themesArray[i] ?? '').replace('//', '') === themePath.replace('//', '')) {
          if (!(themesArray[i] ?? '').startsWith('//')) {
            newElement = '//' + (themesArray[i] ?? '');
            themesArray.splice(i, 1);
            themesArray.unshift(newElement);
            const disabled = document.getElementById('disabled-themes');
            if (disabled && themeContainer) disabled.prepend(themeContainer);

            this.classList.remove('disable-theme-hitbox');
            this.classList.add('enable-theme-hitbox');

            arrow?.classList.remove('disable-theme');
            arrow?.classList.add('enable-theme');

            upDownArrows?.classList.add('invisible');
          } else {
            newElement = (themesArray[i] ?? '').replace('//', '');
            themesArray.splice(i, 1);
            themesArray.unshift(newElement);
            const enabled = document.getElementById('enabled-themes');
            if (enabled && themeContainer) enabled.prepend(themeContainer);

            this.classList.remove('enable-theme-hitbox');
            this.classList.add('disable-theme-hitbox');

            arrow?.classList.remove('enable-theme');
            arrow?.classList.add('disable-theme');

            upDownArrows?.classList.remove('invisible');
          }
        }
      }
      void newElement;

      console.log(themesArray);

      updateThemesInputValue(themesArray);
    });
  });

  const upDownArrows = document.querySelectorAll('.arrow-up-hitbox, .arrow-down-hitbox');
  upDownArrows.forEach((arrow) => {
    arrow.addEventListener('click', handleArrowClick);
  });
}

console.log('themes-setting.js loaded');
