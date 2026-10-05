import registry from './icon-registry.json';
export type IconCategory = 'media' | 'devices' | 'ui' | 'folders' | 'system';
export type RegisteredIcon = {
  path: string;
  category?: string;
  stroke?: string;
  fill?: string;
  background?: string;
};
const icons: Record<string, RegisteredIcon> = registry.icons;
const aliases: Record<string, string> = registry.aliases;
const iconNames = Object.keys(icons);
export function iconKey(name: string): string {
  return Object.hasOwn(aliases, name) ? aliases[name]! : name.replace(/^icon:/, '');
}
export function registeredIcon(name: string): RegisteredIcon | undefined {
  return Object.hasOwn(icons, iconKey(name)) ? icons[iconKey(name)] : undefined;
}
export function matchingIcons(query: string): string[] {
  return iconNames.filter((name) => name.includes(query.trim().toLowerCase()));
}
export function iconCategory(name: string): IconCategory {
  const category = registeredIcon(name)?.category;
  return category === 'media' ||
    category === 'devices' ||
    category === 'folders' ||
    category === 'system'
    ? category
    : 'ui';
}
