#!/usr/bin/env python3
"""Exercise the real Pi CLI in a disposable, credential-free agent directory."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

binary = Path('target/debug/pi-profile' + ('.exe' if os.name == 'nt' else '')).resolve()
with tempfile.TemporaryDirectory(prefix='pi-profile-e2e-') as temp:
    work = Path(temp).resolve()
    agent = work / 'agent space 中文'
    agent.mkdir()
    spec = 'npm:pi-context-usage@' + os.environ['USAGE_VERSION'].strip()
    entry = {'source': spec, 'extensions': []}
    (agent / 'settings.json').write_text(json.dumps({'packages': [entry], 'preservedSentinel': True}), encoding='utf-8')
    answers = json.loads(Path('examples/answers.json').read_text(encoding='utf-8'))
    answers.update(network='direct', model=None, update_pi=False)
    answer_file = work / 'answers.json'
    answer_file.write_text(json.dumps(answers), encoding='utf-8')
    def run(*args):
        subprocess.run([str(binary), '--agent-dir', str(agent), *args], check=True, stdin=subprocess.DEVNULL)
    run('configure', '--answers', str(answer_file), '--yes')
    run('doctor', '--strict')
    settings = json.loads((agent / 'settings.json').read_text(encoding='utf-8'))
    assert settings['preservedSentinel'] is True
    assert entry in settings['packages'], settings['packages']
    package = agent / 'npm/node_modules/pi-context-usage/package.json'
    assert json.loads(package.read_text(encoding='utf-8'))['version'] == os.environ['USAGE_VERSION'].strip()
    run('update', '--yes')
    answers['packages']['usage'] = 'disable'
    answer_file.write_text(json.dumps(answers), encoding='utf-8')
    run('configure', '--answers', str(answer_file), '--yes', '--offline')
    settings = json.loads((agent / 'settings.json').read_text(encoding='utf-8'))
    assert not any((x if isinstance(x, str) else x['source']).startswith('npm:pi-context-usage') for x in settings['packages'])
    assert package.is_file(), 'Detaching a package must not delete its cache'
    run('doctor', '--strict')
    print('REAL PI INTEGRATION PASS: fresh install, pin, resource filters, repeat update, detach and metadata checks')
