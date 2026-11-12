import policy from './coverage-policy.json' with { type: 'json' };
import { flattenMessages } from '../../frontend/locales/schema.ts';

// Only reviewed protocol names, brands, shared words and placeholder-only formats
// may equal English. Generic runtime dictionaries can be partial; the catalogue
// reader requires complete coverage for every shipped locale.
export function auditCoverage(catalogs, messages, rules = policy) {
  return [...catalogs].map(([locale, catalog]) => {
    const own = flattenMessages(catalog);
    const missing = Object.keys(messages).filter((key) => !Object.hasOwn(own, key));
    const unchanged =
      locale === 'en_US'
        ? []
        : Object.keys(own).filter((key) => !key.startsWith('meta.') && own[key] === messages[key]);
    const reviewed = new Set(rules.allowUnchanged[locale] ?? []);
    return {
      locale,
      status: rules.complete.includes(locale) ? 'complete' : 'partial',
      authored: Object.keys(own).length,
      total: Object.keys(messages).length,
      missing,
      unchanged,
      unreviewedUnchanged: unchanged.filter((key) => !reviewed.has(key)),
    };
  });
}

export function validateCoverage(catalogs, messages, rules = policy) {
  const reports = auditCoverage(catalogs, messages, rules);
  const errors = rules.complete
    .filter((locale) => !catalogs.has(locale))
    .map((locale) => `Missing complete locale: ${locale}`);
  for (const report of reports) {
    if (report.status === 'complete' && report.missing.length)
      errors.push(`${report.locale}: missing translations: ${report.missing.join(', ')}`);
    if (report.unreviewedUnchanged.length)
      errors.push(
        `${report.locale}: unreviewed English text: ${report.unreviewedUnchanged.join(', ')}`,
      );
  }
  if (errors.length) throw Error(errors.join('\n'));
  return reports;
}
