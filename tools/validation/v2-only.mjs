// A migration completion gate, deliberately scoped to production sources.
// Rejection tests and historical documents may mention retired interfaces.
import fs from 'node:fs';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '../..');
const rules = [
  ['legacy parser', /\bparse_legacy\b/],
  ['compatibility facade', /crate::app::buttons\b/],
  ['string command request', /pub message: String|message:\s*string/],
  ['legacy command delimiter', /<\|§\|>/],
  ['retired socket event', /message_from_socket|json_data|command_error/],
  ['API generation switch', /strict:\s*bool|legacy:\s*bool|\blegacy\s*&&/],
  ['legacy authorization', /["']legacy["']/],
  ['v1 config conversion', /config\.v1\.backup|fn migrate\(|parse_stringified_list|check_config_hyphen_case|check_config_booleans/],
  ['retired route', /["'`]\/(?:send-data|save_config|COMPLETE_save_config|save_single_button|save_buttons_only|get_config|create_folder|api\/boot|usage|upload_file|upload_filepath|upload_folderpath)(?:["'`]|\b)/],
  ['legacy asset reference', /\.config\/|\*\*uploaded\//],
];
const directories = ['src', 'frontend/src', 'frontend/e2e/demo', 'tools/validation'];
const failures = [];
function scan(directory) {
  for (const entry of fs.readdirSync(directory, {withFileTypes:true})) {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) { scan(target); continue; }
    if (!/\.(rs|ts|svelte|mjs|py)$/.test(entry.name) || /\.(test|spec)\./.test(entry.name) || target === import.meta.filename) continue;
    // Rust unit-test modules may explicitly exercise rejection fixtures.
    const contents = fs.readFileSync(target, 'utf8').split('#[cfg(test)]')[0];
    for (const [index, line] of contents.split('\n').entries()) {
      // Comments are not production callers. All executable references count.
      if (/^\s*(?:\/\/|\*|#)/.test(line)) continue;
      for (const [name, pattern] of rules) {
        const relative = path.relative(root,target).replaceAll('\\', '/');
        if (name === 'string command request' && !['src/domain/transport.rs', 'frontend/src/contracts/v2.ts'].includes(relative)) continue;
        if (name === 'retired route' && !/^(src\/app\/server\/|frontend\/src\/api\/|frontend\/e2e\/demo\/|tools\/validation\/)/.test(relative)) continue;
        if (pattern.test(line)) failures.push({surface:name, file:path.relative(root,target), line:index+1});
      }
    }
  }
}
for (const directory of directories) scan(path.join(root,directory));
const report = {complete:failures.length === 0, violations:failures};
if (process.argv.includes('--json')) console.log(JSON.stringify(report,null,2));
else {
  for (const f of failures) console.error(`${f.file}:${f.line}: ${f.surface}`);
  console.log(`V2-only migration guard: ${failures.length} production violations`);
}
process.exitCode = failures.length ? 1 : 0;
