#!/usr/bin/env python3
"""Build-time packaging only. Runtime is the Rust binary."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile

target = sys.argv[1]
version = tomllib.loads(Path('Cargo.toml').read_text())['package']['version']
name = f'pi-profile-v{version}-{target}'
binary = 'pi-profile.exe' if 'windows' in target else 'pi-profile'
source = Path('target') / target / 'release' / binary
assert source.is_file(), source
out = Path('dist'); out.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory() as tmp:
    folder = Path(tmp) / name; folder.mkdir()
    shutil.copy2(source, folder / binary)
    (folder / binary).chmod(0o755)
    for filename in ['README.md', 'LICENSE', 'AGENTS.md', 'APPEND_SYSTEM.md']:
        shutil.copy2(filename, folder / filename)
    shutil.copytree('examples', folder / 'examples')
    shutil.copytree('docs', folder / 'docs')
    info = {'version': version, 'target': target, 'source_sha': os.environ['SOURCE_SHA'],
            'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip()}
    (folder / 'BUILD-INFO.json').write_text(json.dumps(info, indent=2) + '\n')
    entries = sorted(p for p in folder.rglob('*') if p.is_file())
    (folder / 'CONTENTS.sha256').write_text(''.join(
        f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(folder).as_posix()}\n' for p in entries))
    if 'windows' in target:
        archive = out / f'{name}.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
            for p in sorted(folder.rglob('*')):
                if p.is_file(): z.write(p, p.relative_to(folder.parent).as_posix())
    else:
        archive = out / f'{name}.tar.gz'
        with tarfile.open(archive, 'w:gz') as tar: tar.add(folder, arcname=name)
    print(archive, hashlib.sha256(archive.read_bytes()).hexdigest())
