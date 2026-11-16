#!/usr/bin/env python3
"""Verify the public revision and every immutable resource, without a browser cache."""
import argparse
import concurrent.futures
import hashlib
import json
import time
import urllib.request
from pathlib import Path


def fetch(url):
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=90) as response:
                return response.read()
        except OSError:
            if attempt == 2:
                raise
            time.sleep(attempt + 1)


def verify(base, revision):
    base = base.rstrip('/') + '/'
    manifest = json.loads(fetch(base + 'asset-manifest.json'))
    receipt = json.loads(fetch(base + 'build.json'))
    if receipt['revision'] != revision or manifest['revision'] != revision:
        raise ValueError(f'Public source mismatch: {receipt["revision"]}, {manifest["revision"]}; expected {revision}')
    worker = fetch(base + 'sw.js').decode()
    if json.dumps(manifest, separators=(',', ':')) not in worker:
        raise ValueError('Worker does not contain the published manifest')

    def resource(entry):
        if not entry['url'].startswith('_resources/' + entry['sha256'] + '/'):
            raise ValueError(f'Non-immutable resource URL: {entry["path"]}')
        payload = fetch(base + entry['url'])
        if len(payload) != entry['bytes'] or hashlib.sha256(payload).hexdigest() != entry['sha256']:
            raise ValueError(f'Resource integrity mismatch: {entry["path"]}')
        return {'path': entry['path'], 'bytes': len(payload), 'sha256': entry['sha256']}

    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        resources = list(pool.map(resource, manifest['files']))
    return {'passed': True, 'base': base, 'revision': revision, 'version': manifest['version'],
            'engine': receipt, 'verified_files': len(resources), 'resources': resources}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--base', default='https://stars-labs.github.io/bevy-open-arpg/')
    parser.add_argument('--revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = verify(args.base, args.revision)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(f'PASS {result["revision"]}: {result["verified_files"]} immutable resources verified')
