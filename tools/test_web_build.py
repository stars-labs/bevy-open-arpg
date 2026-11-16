"""Exercise the real packaging wrapper without invoking expensive Rust compilation."""
import functools
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import unittest

ROOT=Path(__file__).resolve().parents[1]
class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self,*_): pass

class BuildTests(unittest.TestCase):
    def test_content_update_reuses_engine_without_compilation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)/'project'
            for directory in ['scripts','tools','src','web/icons','assets/fonts','fakebin']:
                (root/directory).mkdir(parents=True)
            for name in ['scripts/build_web.sh','scripts/stamp_web_build_id.py','tools/web_engine_inputs.py','tools/reuse_web_engine.py',
                         'tools/generate_web_manifest.py','tools/retain_web_releases.py','tools/sw-template.js']:
                shutil.copy2(ROOT/name,root/name)
            (root/'Cargo.toml').write_text('[package]\nname="fixture"')
            (root/'Cargo.lock').write_text('[[package]]\nname = "wasm-bindgen"\nversion = "0.2.126"\n')
            (root/'src/main.rs').write_text('fn main() {}')
            (root/'web/index.html').write_text('<html><script>const buildId = "fixture";</script> original</html>')
            (root/'web/loading.js').write_text('export const ready=true;')
            for name in ['game-cache.js','game-cache.css','downloads.js','pwa.js','pwa.css']:
                shutil.copyfile(ROOT/'web'/name,root/'web'/name)
            (root/'web/manifest.webmanifest').write_text('{}')
            (root/'web/icons/icon.png').write_bytes(b'icon')
            (root/'assets/fonts/font.otf').write_bytes(b'font')
            programs={
                'git':'#!/usr/bin/env sh\necho fixture-revision\n',
                'cargo':'''#!/usr/bin/env python3
from pathlib import Path
import os,sys
if sys.argv[1]=='tree':sys.exit(0)
with open(os.environ['FIXTURE_LOG'],'a') as f:f.write('cargo\\n')
p=Path('target/wasm32-unknown-unknown/release/bevy-open-arpg.wasm');p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(bytes([0,97,115,109,1,0,0,0]))
''',
                'wasm-bindgen':'''#!/usr/bin/env python3
from pathlib import Path
import sys,shutil,os
if '--version' in sys.argv:print('wasm-bindgen 0.2.126');sys.exit(0)
with open(os.environ['FIXTURE_LOG'],'a') as f:f.write('bindgen\\n')
shutil.copyfile('target/wasm32-unknown-unknown/release/bevy-open-arpg.wasm','web/pkg/bevy_open_arpg_bg.wasm')
Path('web/pkg/bevy_open_arpg.js').write_text('export default function init() {}')

''',
                'wasm-opt':'''#!/usr/bin/env python3
import sys,shutil
if '--version' in sys.argv:print('wasm-opt version 130');sys.exit(0)
shutil.copyfile(sys.argv[-1],sys.argv[sys.argv.index('-o')+1])
''',
            }
            for name,script in programs.items():
                p=root/'fakebin'/name;p.write_text(script);p.chmod(0o755)
            (root/'fakecargo/bin').mkdir(parents=True)
            shutil.copy2(root/'fakebin/wasm-bindgen',root/'fakecargo/bin/wasm-bindgen')
            shutil.copy2(root/'fakebin/wasm-opt',root/'fakecargo/bin/wasm-opt')
            empty=Path(temporary)/'empty';empty.mkdir()
            initial_server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Quiet,directory=str(empty)))
            initial_thread=threading.Thread(target=initial_server.serve_forever,daemon=True);initial_thread.start()
            env=dict(os.environ,PATH=str(root/'fakebin')+os.pathsep+os.environ['PATH'],
                     CARGO_HOME=str(root/'fakecargo'),FIXTURE_LOG=str(root/'calls.log'),WASM_REUSE_BASE_URL=f'http://127.0.0.1:{initial_server.server_port}/')
            env.pop('RUSTFLAGS',None)
            def build():
                result=subprocess.run(['bash','scripts/build_web.sh'],cwd=root,env=env,capture_output=True,text=True,timeout=45)
                self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            try: build()
            finally:
                initial_server.shutdown();initial_server.server_close();initial_thread.join()
            self.assertIn('cargo',(root/'calls.log').read_text())
            engine={name:(root/'web'/name).read_bytes() for name in ['pkg/bevy_open_arpg.js','pkg/bevy_open_arpg_bg.wasm','pkg/bevy_open_arpg_bg.wasm.gz']}
            published=Path(temporary)/'published';shutil.copytree(root/'web',published)
            server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Quiet,directory=str(published)))
            thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
            try:
                env['WASM_REUSE_BASE_URL']=f'http://127.0.0.1:{server.server_port}/'
                (root/'web/index.html').write_text('<html><script>const buildId = "fixture";</script> updated</html>')
                (root/'calls.log').write_text('')
                build()
                self.assertEqual((root/'calls.log').read_text(),'')
                for name,data in engine.items():self.assertEqual((root/'web'/name).read_bytes(),data)
                self.assertIn('updated',(root/'web/index.html').read_text())
                report=json.loads((root/'web/update-size-report.json').read_text())
                self.assertTrue(report)
                (root/'src/main.rs').write_text('fn main() { println!("changed"); }')
                build()
                self.assertIn('cargo',(root/'calls.log').read_text())
            finally:
                server.shutdown();server.server_close();thread.join()

if __name__=='__main__':unittest.main()
