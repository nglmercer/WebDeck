import { readCatalogs } from './catalog.mjs';
import { auditCoverage } from './coverage.mjs';
const { catalogs, messages } = await readCatalogs();
const reports = auditCoverage(catalogs, messages);
if (process.argv.includes('--json')) console.log(JSON.stringify(reports, null, 2));
else
  for (const report of reports)
    console.log(
      `${report.locale}: ${report.authored}/${report.total} authored (${report.status}); ${report.missing.length} fallback leaves; ${report.unreviewedUnchanged.length} unreviewed English strings`,
    );
