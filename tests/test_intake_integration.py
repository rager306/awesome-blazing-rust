"""Opt-in offline workflow smoke tests.

Run INTAKE_INTEGRATION=1 python -m unittest discover -s tests -p test_intake_integration.py.
These execute actual workflow shell bodies, not a GitHub Actions runner. They
need PyYAML, git, jq, a configured cargo, and an offline-cached awesome-lint.
"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
CARD = '- [example](https://github.com/example/project) - sample project. **Blazing:** claimed fast. **Type:** case-study. **Arch:** `zero-copy`.'

# The only external services replaced are GitHub, Claude, and artifact transfer.
GH = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path
sfile = Path(os.environ['MOCK_STATE'])
s = json.loads(sfile.read_text())
a = sys.argv[1:]
s['calls'].append(a)
def save(): sfile.write_text(json.dumps(s))
def output(x): print(json.dumps(x) if not isinstance(x, str) else x)
if a[:2] == ['issue', 'view']:
    field = a[a.index('--jq') + 1].lstrip('.')
    output(s['issue'][field])
elif a[:2] == ['issue', 'comment'] or a[:2] == ['issue', 'close']:
    pass
elif a[0] == 'api':
    path = a[1]
    if path.startswith('repos/org/catalog/pulls?'):
        output([s['pulls']])
    elif path.startswith('repos/org/catalog/branches?'):
        refs = subprocess.check_output(['git', '--git-dir', os.environ['MOCK_REMOTE'], 'for-each-ref', '--format=%(refname:strip=2)', 'refs/heads/'], text=True).splitlines()
        output([[{'name': ref} for ref in refs]])
    elif path.startswith('repos/org/catalog/issues/'):
        output(s['issue'])
    elif path == 'repos/example/project':
        output(s['repo'])
    else:
        print('unexpected API: ' + path, file=sys.stderr); sys.exit(1)
elif a[:2] == ['pr', 'create']:
    body = Path(a[a.index('--body-file') + 1]).read_text()
    head = a[a.index('--head') + 1]
    s['pulls'].append({'number': 101, 'body': body, 'head': {'ref': head, 'repo': {'full_name': 'org/catalog'}}, 'labels': [{'name': 'card-draft'}]})
elif a[:2] == ['pr', 'view']:
    output('101')
else:
    print('unexpected gh: ' + repr(a), file=sys.stderr); sys.exit(1)
save()
'''
CLAUDE = r'''#!/usr/bin/env python3
import json, os
print(json.dumps({'result': json.dumps(json.loads(os.environ['MOCK_CARD']))}))
'''


@unittest.skipUnless(os.environ.get('INTAKE_INTEGRATION') == '1', 'set INTAKE_INTEGRATION=1 for offline integration tests')
class IntakeWorkflowIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import yaml
        cls.workflow = yaml.safe_load((ROOT / '.github/workflows/intake.yml').read_text())
        cls.build = tempfile.TemporaryDirectory()
        env = os.environ.copy()
        env['CARGO_TARGET_DIR'] = cls.build.name
        subprocess.run(['cargo', 'build', '--release', '--offline', '--manifest-path', str(ROOT / 'cli/Cargo.toml')], check=True, env=env, capture_output=True, text=True)

    @classmethod
    def tearDownClass(cls):
        cls.build.cleanup()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.remote = self.base / 'remote.git'
        self.work = self.base / 'work'
        self.bin = self.base / 'bin'
        self.bin.mkdir()
        for name, content in [('gh', GH), ('claude', CLAUDE)]:
            path = self.bin / name
            path.write_text(content)
            path.chmod(0o755)
        self.statefile = self.base / 'state.json'
        self.state = {'calls': [], 'pulls': [], 'issue': {'body': '### Repository URL\n\nhttps://github.com/example/project\n', 'title': 'Example', 'state': 'OPEN'}, 'repo': {'id': 42, 'full_name': 'example/project', 'html_url': 'https://github.com/example/project', 'archived': False, 'pushed_at': '2099-01-01T00:00:00Z', 'stargazers_count': 1, 'license': 'MIT', 'language': 'Rust'}}
        self.save()
        subprocess.run(['git', 'init', '--bare', str(self.remote)], check=True, capture_output=True)
        shutil.copytree(ROOT, self.work, ignore=shutil.ignore_patterns('.git', 'target', '__pycache__', 'node_modules'))
        self.git('init', '-b', 'main')
        self.git('config', 'user.name', 'Integration test')
        self.git('config', 'user.email', 'integration@example.invalid')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')
        self.git('remote', 'add', 'origin', str(self.remote))
        self.git('push', '-q', '-u', 'origin', 'main')
        # The linter reads the push URL; actual pushes are reset to the local
        # bare repository before the workflow's Open draft PR step.
        self.git('config', 'remote.origin.pushurl', 'https://github.com/org/catalog.git')
        (self.work / 'cli/target').symlink_to(self.build.name, target_is_directory=True)
        self.env = os.environ.copy()
        self.env.update({'PATH': str(self.bin) + os.pathsep + self.env['PATH'], 'MOCK_STATE': str(self.statefile), 'MOCK_REMOTE': str(self.remote), 'ISSUE_NUMBER': '77', 'TARGET_REPOSITORY': 'org/catalog', 'CARGO_NET_OFFLINE': 'true', 'CARGO_TARGET_DIR': self.build.name, 'npm_config_offline': 'true', 'HTTP_PROXY': 'http://127.0.0.1:9', 'HTTPS_PROXY': 'http://127.0.0.1:9', 'ALL_PROXY': 'http://127.0.0.1:9', 'NO_PROXY': '', 'GITHUB_OUTPUT': str(self.base / 'output'), 'GITHUB_ENV': str(self.base / 'env'), 'MOCK_CARD': json.dumps({'skip': None, 'card': CARD, 'what': 'What', 'why': 'Why', 'weak': 'Weak'})})
        self.out = Path(self.env['GITHUB_OUTPUT'])
        self.github_env = Path(self.env['GITHUB_ENV'])
        self.values = {'github.server_url': 'https://github.com', 'github.repository': 'org/catalog', 'github.run_id': '123', 'github.run_attempt': '1'}

    def save(self):
        self.statefile.write_text(json.dumps(self.state))

    def load(self):
        self.state = json.loads(self.statefile.read_text())

    def git(self, *args, cwd=None):
        return subprocess.run(['git', *args], cwd=cwd or self.work, check=True, capture_output=True, text=True).stdout.strip()

    def step(self, job, name, *, check=True):
        step = next(s for s in self.workflow['jobs'][job]['steps'] if s.get('name') == name)
        script = step['run']
        script = re.sub(r'\$\{\{\s*([^}]+?)\s*\}\}', lambda m: self.values[m.group(1).strip()], script)
        self.out.write_text('')
        result = subprocess.run(['bash', '-e', '-o', 'pipefail', '-c', script], cwd=self.work, env=self.env, capture_output=True, text=True)
        if check:
            logs = '\n'.join(f'{p.name}:\n{p.read_text()}' for p in self.work.glob('gate-*.log'))
            self.assertEqual(result.returncode, 0, f'{job}: {name}\n{result.stdout}\n{result.stderr}\n{logs}')
        outputs = dict(line.split('=', 1) for line in self.out.read_text().splitlines() if '=' in line)
        self.load()
        return result, outputs

    def draft(self):
        _, gate = self.step('draft-card', 'Freshness and duplicate gate (pipeline-side, deterministic)')
        self.env.update({'REPO_SLUG': gate['repo'], 'REPO_ID': gate['repo_id'], 'REPO_META': gate.get('meta_json', '')})
        if gate.get('skip'):
            return gate['skip']
        result, pre = self.step('draft-card', 'Check for an existing draft before research', check=False)
        if result.returncode:
            return 'error'
        return pre['exists']

    def research(self):
        self.step('research', 'Research and draft the card text')
        _, result = self.step('research', 'Set research artifact name')
        self.assertEqual(result['name'], 'intake-research-123-1')
        # Model upload/download with a separate artifact store.
        self.artifact = self.base / 'card.json'
        shutil.copy2(self.work / 'card.json', self.artifact)
        (self.work / 'card.json').unlink()

    def finalize(self):
        self.step('finalize', 'Refresh main for finalization')
        self.env['ARTIFACT_NAME'] = 'intake-research-123-1'
        self.step('finalize', 'Check research artifact reference')
        shutil.copy2(self.artifact, self.work / 'card.json')
        self.step('finalize', 'Confirm issue is open')
        result, existing = self.step('finalize', 'Resolve existing draft safely', check=False)
        if result.returncode:
            return 'error'
        if existing['exists'] == 'true':
            return 'duplicate'
        self.step('finalize', 'Insert the card and run the gates')
        if self.github_env.exists() and 'skip=agent' in self.github_env.read_text():
            return 'skip'
        self.git('config', 'remote.origin.pushurl', str(self.remote))
        self.step('finalize', 'Open draft PR')
        return 'opened'

    def test_new_pr_and_preflight_duplicate(self):
        self.assertEqual(self.draft(), 'false')
        self.research()
        self.assertEqual(self.finalize(), 'opened')
        self.assertEqual(self.git('config', '--get', 'branch.agent/card-repo-42.remote'), 'origin')
        self.assertEqual(self.git('config', '--get', 'branch.agent/card-repo-42.merge'), 'refs/heads/main')
        self.assertEqual(len(self.state['pulls']), 1)
        self.assertIn(CARD, self.git('show', 'origin/agent/card-repo-42:README.md'))
        # The next run checks out main, not the previous run's candidate branch.
        self.git('checkout', '--detach', 'origin/main')
        self.assertEqual(self.draft(), 'true')
        self.assertEqual(len(self.state['pulls']), 1)

    def test_orphan_branch_refuses_to_overwrite(self):
        self.git('push', str(self.remote), 'HEAD:refs/heads/agent/card-repo-42')
        before = self.git('rev-parse', 'refs/heads/agent/card-repo-42', cwd=self.remote)
        self.assertEqual(self.draft(), 'error')
        self.assertEqual(before, self.git('rev-parse', 'refs/heads/agent/card-repo-42', cwd=self.remote))
        self.assertFalse(self.state['pulls'])

    def test_agent_skip_keeps_issue_open(self):
        self.assertEqual(self.draft(), 'false')
        self.env['MOCK_CARD'] = json.dumps({'skip': 'no speed claim', 'card': '', 'what': '', 'why': '', 'weak': ''})
        self.research()
        self.assertEqual(self.finalize(), 'skip')
        self.assertFalse(self.state['pulls'])
        self.assertTrue(any(c[:2] == ['issue', 'comment'] for c in self.state['calls']))
        self.assertFalse(any(c[:2] == ['issue', 'close'] for c in self.state['calls']))

    def test_delayed_finalization_refreshes_main(self):
        self.assertEqual(self.draft(), 'false')
        self.research()
        other = self.base / 'other'
        subprocess.run(['git', 'clone', '-q', '-b', 'main', str(self.remote), str(other)], check=True)
        self.git('config', 'user.name', 'Other', cwd=other)
        self.git('config', 'user.email', 'other@example.invalid', cwd=other)
        with (other / 'README.md').open('a') as f:
            f.write('\n<!-- newer main -->\n')
        self.git('add', 'README.md', cwd=other)
        self.git('commit', '-qm', 'advance main', cwd=other)
        new_head = self.git('rev-parse', 'HEAD', cwd=other)
        self.git('push', '-q', 'origin', 'main', cwd=other)
        self.assertEqual(self.finalize(), 'opened')
        self.assertEqual(self.git('rev-parse', 'origin/agent/card-repo-42^'), new_head)
        self.assertIn('<!-- newer main -->', self.git('show', 'origin/agent/card-repo-42:README.md'))

    def test_duplicate_appearing_during_research_blocks_finalization(self):
        self.assertEqual(self.draft(), 'false')
        self.research()
        self.state['pulls'].append({'number': 90, 'body': 'Repository: https://github.com/example/project\n<!-- abr-repo-id: 42 -->', 'head': {'ref': 'agent/card-repo-42', 'repo': {'full_name': 'org/catalog'}}, 'labels': [{'name': 'card-draft'}]})
        self.save()
        self.assertEqual(self.finalize(), 'duplicate')
        self.assertEqual(len(self.state['pulls']), 1)
        self.assertFalse(any(c[:2] == ['pr', 'create'] for c in self.state['calls']))


if __name__ == '__main__':
    unittest.main()
