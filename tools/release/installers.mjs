// Build unsigned preliminary installers from the already verified native portable archive.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, cpSync, symlinkSync, rmSync, chmodSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
const root = path.resolve(import.meta.dirname, '../..');
const platform = process.platform === 'win32' ? 'windows' : process.platform === 'darwin' ? 'macos' : 'linux';
const arch = process.arch === 'arm64' ? 'aarch64' : 'x86_64';
const version = readFileSync(path.join(root, 'Cargo.toml'), 'utf8').match(/^version = "([\w.+-]+)"/m)?.[1];
if (!version) throw Error('Missing application version');
const dev = process.argv.includes('--dev');
const name = `WebDeck-${version}-${platform}-${arch}${dev ? '-dev' : ''}`;
const output = path.join(root, 'dist');
const archive = path.join(output, `${name}-portable.zip`);
const hash = createHash('sha256').update(readFileSync(archive)).digest('hex');
if (!readFileSync(`${archive}.sha256`, 'utf8').startsWith(hash)) throw Error('Portable checksum mismatch');
const directory = mkdtempSync(path.join(tmpdir(), 'webdeck-installer-'));
const python = process.platform === 'win32' ? 'python' : 'python3';
const run = (cmd, args) => execFileSync(cmd, args, { stdio: 'inherit' });
let artifact;
try {
  run(python, ['-c', `import zipfile,pathlib,sys,stat
root=pathlib.Path(sys.argv[2])
with zipfile.ZipFile(sys.argv[1]) as z:
 for e in z.infolist():
  p=pathlib.PurePosixPath(e.filename);mode=e.external_attr>>16
  assert not p.is_absolute() and '..' not in p.parts and p.parts[0]=='WebDeck' and not stat.S_ISLNK(mode)
  t=root.joinpath(*p.parts)
  if e.is_dir():t.mkdir(parents=True,exist_ok=True)
  else:t.parent.mkdir(parents=True,exist_ok=True);t.write_bytes(z.read(e));t.chmod(0o755 if mode&0o111 else 0o644)`, archive, directory]);
  const payload = path.join(directory, 'WebDeck');
  const icon = path.join(directory, 'icon.png');
  run(python, [path.join(root, 'tools/release/icon.py'), icon]);
  if (platform === 'linux') {
    const staging = path.join(directory, 'deb');
    mkdirSync(path.join(staging, 'DEBIAN'), { recursive: true });
    chmodSync(path.join(staging, 'DEBIAN'), 0o755);
    cpSync(payload, path.join(staging, 'opt/webdeck'), { recursive: true });
    mkdirSync(path.join(staging, 'usr/bin'), { recursive: true });
    writeFileSync(path.join(staging, 'usr/bin/webdeck'), '#!/bin/sh\ncd /opt/webdeck || exit 1\nexec ./WebDeck --config-dir "${XDG_CONFIG_HOME:-$HOME/.config}/webdeck" "$@"\n', { mode: 0o755 });
    mkdirSync(path.join(staging, 'usr/share/applications'), { recursive: true });
    mkdirSync(path.join(staging, 'usr/share/icons/hicolor/1024x1024/apps'), { recursive: true });
    cpSync(icon, path.join(staging, 'usr/share/icons/hicolor/1024x1024/apps/webdeck.png'));
    writeFileSync(path.join(staging, 'usr/share/applications/webdeck.desktop'), '[Desktop Entry]\nType=Application\nName=WebDeck\nComment=Desktop control deck\nExec=webdeck\nIcon=webdeck\nTerminal=false\nCategories=Utility;\n');
    writeFileSync(path.join(staging, 'DEBIAN/control'), `Package: webdeck\nVersion: ${version.replace('-', '~')}\nArchitecture: ${arch === 'aarch64' ? 'arm64' : 'amd64'}\nMaintainer: WebDeck contributors\nDepends: libgtk-3-0t64 | libgtk-3-0, libasound2t64 | libasound2, libxcb1, libxcb-render0, libxcb-shape0, libxcb-xfixes0, libxkbcommon0\nDescription: WebDeck v2 preliminary desktop controller${dev ? ' (development build)' : ''}\n`);
    artifact = path.join(output, `${name}.deb`);
    run('dpkg-deb', ['--build', '--root-owner-group', staging, artifact]);
    run('dpkg-deb', ['--info', artifact]);
  } else if (platform === 'macos') {
    const volume = path.join(directory, 'volume');
    const app = path.join(volume, 'WebDeck.app/Contents');
    mkdirSync(path.join(app, 'MacOS'), { recursive: true });
    mkdirSync(path.join(app, 'Resources'), { recursive: true });
    cpSync(payload, path.join(app, 'Resources/WebDeck'), { recursive: true });
    writeFileSync(path.join(app, 'MacOS/launcher'), '#!/bin/sh\ncd "$(dirname "$0")/../Resources/WebDeck" || exit 1\nexec ./WebDeck --config-dir "$HOME/Library/Application Support/WebDeck" "$@"\n', { mode: 0o755 });
    const iconset = path.join(directory, 'WebDeck.iconset');
    mkdirSync(iconset);
    for (const size of [16, 32, 128, 256, 512]) {
      run('sips', ['-z', String(size), String(size), icon, '--out', path.join(iconset, `icon_${size}x${size}.png`)]);
      run('sips', ['-z', String(size * 2), String(size * 2), icon, '--out', path.join(iconset, `icon_${size}x${size}@2x.png`)]);
    }
    run('iconutil', ['-c', 'icns', iconset, '-o', path.join(app, 'Resources/WebDeck.icns')]);
    writeFileSync(path.join(app, 'Info.plist'), `<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleName</key><string>WebDeck</string><key>CFBundleIdentifier</key><string>org.webdeck.desktop</string><key>CFBundleExecutable</key><string>launcher</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>${version.split('-')[0]}</string><key>CFBundleVersion</key><string>${version.split('-')[0]}</string><key>CFBundleGetInfoString</key><string>WebDeck ${version}${dev ? ' development' : ''}</string><key>CFBundleIconFile</key><string>WebDeck.icns</string><key>LSMinimumSystemVersion</key><string>12.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>`);
    run('plutil', ['-lint', path.join(app, 'Info.plist')]);
    symlinkSync('/Applications', path.join(volume, 'Applications'));
    artifact = path.join(output, `${name}.dmg`);
    run('hdiutil', ['create', '-ov', '-volname', `WebDeck ${version}`, '-srcfolder', volume, '-format', 'UDZO', artifact]);
    run('hdiutil', ['verify', artifact]);
  } else {
    const script = path.join(directory, 'installer.iss');
    const escape = s => s.replaceAll('"', '""');
    artifact = path.join(output, `${name}-setup.exe`);
    writeFileSync(script, `[Setup]\nAppId=WebDeckV2\nAppName=WebDeck\nAppVersion=${version}\nDefaultDirName={localappdata}\\Programs\\WebDeck\nPrivilegesRequired=lowest\nArchitecturesAllowed=${arch === 'aarch64' ? 'arm64' : 'x64compatible'}\nOutputDir=${output}\nOutputBaseFilename=${name}-setup\nSetupIconFile=${path.join(payload, 'static/icons/icon.ico')}\nCompression=lzma2\nSolidCompression=yes\nUninstallDisplayIcon={app}\\WebDeck.exe\n[Files]\nSource: "${escape(path.join(payload, '*'))}"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion\n[Icons]\nName: "{userprograms}\\WebDeck"; Filename: "{app}\\WebDeck.exe"; WorkingDir: "{app}"; Parameters: "--config-dir ""{localappdata}\\WebDeck"""\n[Run]\nFilename: "{app}\\WebDeck.exe"; WorkingDir: "{app}"; Parameters: "--config-dir ""{localappdata}\\WebDeck"""; Description: "Open WebDeck"; Flags: nowait postinstall skipifsilent\n`);
    run(process.env.WEBDECK_ISCC ?? path.join(process.env['ProgramFiles(x86)'] ?? 'C:/Program Files (x86)', 'Inno Setup 6', 'ISCC.exe'), [script]);
  }
  const digest = createHash('sha256').update(readFileSync(artifact)).digest('hex');
  writeFileSync(`${artifact}.sha256`, `${digest}  ${path.basename(artifact)}\n`);
  console.log(`Preliminary installer: ${artifact}`);
} finally { rmSync(directory, { recursive: true, force: true }); }
