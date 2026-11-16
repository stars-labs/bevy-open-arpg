// Launcher acceptance with real HTTP/SW and a deliberately held engine fixture.
// This is not a substitute for actual WebGPU gameplay acceptance.
const {chromium}=require('playwright');
const fs=require('node:fs/promises'),path=require('node:path'),os=require('node:os'),http=require('node:http');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const {gzipSync}=require('node:zlib');
(async()=>{
 const root=path.resolve(__dirname,'..'),dir=await fs.mkdtemp(path.join(os.tmpdir(),'arpg-launcher-'));
 const output=path.join(root,'target/launcher-tests');await fs.mkdir(output,{recursive:true});
 for(const file of ['index.html','styles.css','launcher.js','game-cache.js','game-cache.css','downloads.js','pwa.js','pwa.css','manifest.webmanifest'])await fs.copyFile(path.join(root,'web',file),path.join(dir,file));
 await fs.cp(path.join(root,'web/icons'),path.join(dir,'icons'),{recursive:true});
 await fs.mkdir(path.join(dir,'pkg'));await fs.mkdir(path.join(dir,'assets/models'),{recursive:true});
 await fs.writeFile(path.join(dir,'assets/models/enemy.glb'),'fixture-battle');
 await fs.writeFile(path.join(dir,'pkg/bevy_open_arpg_bg.wasm'),'fixture-engine');
 await fs.writeFile(path.join(dir,'pkg/bevy_open_arpg_bg.wasm.gz'),gzipSync('fixture-engine'));
 await fs.writeFile(path.join(dir,'pkg/bevy_open_arpg.js'),`export default async function(){window.fixtureStarted=true;window.finishFixture=()=>{window.arpgViewState='menu';};}`);
 const generated=spawnSync('python3',['tools/generate_web_manifest.py',dir,'--revision','fixture','--template','tools/sw-template.js'],{cwd:root,encoding:'utf8'});assert.equal(generated.status,0,generated.stderr);
 const requests=[];
 const server=http.createServer(async(req,res)=>{
  const file=decodeURIComponent(new URL(req.url,'http://localhost').pathname).replace(/^\//,'')||'index.html';requests.push(file);
  try{const data=await fs.readFile(path.join(dir,file));res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.html')?'text/html':'application/octet-stream');res.end(data);}catch{res.writeHead(404);res.end();}
 });await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_EXECUTABLE_PATH,args:['--no-sandbox']});
 try{
  const context=await browser.newContext({viewport:{width:667,height:375},hasTouch:true});
  await context.addInitScript(()=>Object.defineProperty(navigator,'gpu',{value:{requestAdapter:async()=>({})},configurable:true}));
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  await page.locator('[data-language="zh-CN"]').click();assert.equal(await page.locator('#start').textContent(),'开始旅程');
  await page.reload();assert.equal(await page.locator('#start').textContent(),'开始旅程');
  await page.locator('#start').click();await page.waitForFunction(()=>window.fixtureStarted);
  assert(await page.locator('#loading').isVisible());
  await page.locator('[data-language="en"]').click();assert.equal(await page.locator('#status').textContent(),'Preparing the menu…');
  assert(!requests.some(p=>p.includes('enemy.glb')),'Menu must not transfer battle assets');
  await page.screenshot({path:path.join(output,'landscape-waiting.png')});
  await page.evaluate(()=>window.finishFixture());await page.locator('#loading').waitFor({state:'hidden'});
  await page.evaluate(()=>window.arpgPrepareChapter(0));await page.locator('#downloads-dialog').waitFor({state:'visible'});
  await page.locator('#downloads-close').click();assert.equal(await page.evaluate(()=>window.arpgChapterDownload.status),'cancelled');
  await page.evaluate(()=>window.arpgPrepareChapter(0));await page.locator('#downloads-confirm').click();
  await page.waitForFunction(()=>window.arpgChapterDownload.status==='ready');
  assert(requests.some(p=>p.includes('enemy.glb')));
  await page.evaluate(()=>window.arpgAssetFailure());assert(await page.locator('#loading').isVisible());assert.equal(await page.locator('#start').textContent(),'Retry');
  await page.setViewportSize({width:390,height:844});await page.screenshot({path:path.join(output,'portrait-retry.png')});
  assert.deepEqual(errors,[]);
  await fs.writeFile(path.join(output,'result.json'),JSON.stringify({passed:['prelaunch language persistence','language during initialization','real readiness gate','no battle requests at menu','cancel and retry chapter','decode failure recovery UI'],requests},null,2));
  console.log('PASS 6 launcher scenarios (fixture engine, actual HTTP/Service Worker)');
 }finally{await browser.close();await new Promise(r=>server.close(r));await fs.rm(dir,{recursive:true,force:true});}
})().catch(e=>{console.error(e);process.exitCode=1;});
