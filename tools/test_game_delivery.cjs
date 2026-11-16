// Real shipped engine acceptance. Requires headed Chromium and a private Xvfb.
const {chromium}=require('playwright');
const {PNG}=require('pngjs');
const fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict');
(async()=>{
 const offline=process.env.OFFLINE==='1', mobile=process.env.MOBILE==='1', portrait=process.env.PORTRAIT==='1';
 const output=path.resolve(process.env.OUTPUT||'target/game-delivery');await fs.mkdir(output,{recursive:true});
 const ctx=await chromium.launchPersistentContext(process.env.PROFILE||'/tmp/arpg-delivery-profile',{
  headless:false,executablePath:process.env.CHROMIUM_EXECUTABLE_PATH,
  viewport:mobile?(portrait?{width:390,height:844}:{width:844,height:390}):{width:1280,height:720},hasTouch:true,isMobile:mobile,
  args:['--ozone-platform=x11','--disable-gpu-watchdog','--disable-backgrounding-occluded-windows','--disable-renderer-backgrounding','--disable-background-timer-throttling','--enable-unsafe-webgpu','--use-angle=vulkan','--use-vulkan=swiftshader','--enable-features=Vulkan','--disable-vulkan-surface']
 });
 const errors=[],requests=[];const page=ctx.pages()[0];page.setDefaultTimeout(60000);
 page.on('pageerror',e=>{if(!e.message.includes('Using exceptions for control flow'))errors.push(e.message);});
 page.on('console',m=>{if(m.type()==='error')console.log('CONSOLE',m.text());});
 page.on('request',r=>requests.push(r.url()));
 async function capture(name){
  const image=await page.screenshot({path:path.join(output,name+'.png'),timeout:90000});
  const png=PNG.sync.read(image),colors=new Set();let min=255,max=0;
  for(let y=40;y<png.height-40;y+=8)for(let x=40;x<png.width-40;x+=8){const p=(y*png.width+x)*4;colors.add([png.data[p],png.data[p+1],png.data[p+2]].join(','));min=Math.min(min,png.data[p]);max=Math.max(max,png.data[p]);}
  assert(colors.size>32 && max-min>40,`Rendered ${name} is blank/flat (${colors.size} colors)`);
 }
 try{
  if(offline)await ctx.setOffline(true);
  await page.goto(process.env.URL||'http://127.0.0.1:8786/');
  await page.locator('#start').click();
  await page.waitForFunction(()=>window.arpgViewState==='menu',null,{timeout:240000,polling:100});
  await page.locator('#loading').waitFor({state:'hidden'});await page.waitForTimeout(3500);
  await capture('menu');
  const dimensions=await page.evaluate(()=>{const c=document.querySelector('canvas'),r=c.getBoundingClientRect();return {width:c.width,height:c.height,cssWidth:r.width,cssHeight:r.height,dpr:Math.round(devicePixelRatio*1000)/1000};});
  if(process.env.VIEWPORT==='1')assert(Math.abs(dimensions.width-dimensions.cssWidth*dimensions.dpr)<=1&&Math.abs(dimensions.height-dimensions.cssHeight*dimensions.dpr)<=1,JSON.stringify(dimensions));
  assert(!requests.some(url=>new URL(url).pathname.includes('/assets/models/')), 'Menu must not read combat models');
  await fs.writeFile(path.join(output,'menu.json'),JSON.stringify({errors,requests,offline,mobile,dimensions},null,2));
  if(process.env.MODE==='settings'){
   await page.touchscreen.tap(page.viewportSize().width-42,42);await page.waitForTimeout(1200);await capture('settings');assert.deepEqual(errors,[]);return;
  }
  if(process.env.MODE==='menu'){assert.deepEqual(errors,[]);return;}
  // Existing New Journey entry, followed by the real class selector.
  if(mobile)await page.touchscreen.tap(Number(process.env.START_X||158),Number(process.env.START_Y||(portrait?370:120)));else await page.mouse.click(240,370);await page.waitForTimeout(700);await capture('class-selection');
  if(process.env.MODE==='class'){
   const cdp=await ctx.newCDPSession(page);
   await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:195,y:570,id:5}]});
   await page.waitForTimeout(200);
   for(let y=540;y>=250;y-=30){await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:195,y,id:5}]});await page.waitForTimeout(100);}
   await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
   await page.waitForTimeout(1000);await capture('class-scrolled');assert.equal(await page.evaluate(()=>window.arpgViewState),'menu','Dragging class cards must not start a descent');return;
  }
  if(mobile)await page.touchscreen.tap(Number(process.env.CONFIRM_X||360),Number(process.env.CONFIRM_Y||338));else await page.keyboard.press('Enter');
  await page.waitForFunction(()=>window.arpgChapterDownload?.status==='ready' || (document.querySelector('#downloads-dialog').open && !document.querySelector('#downloads-confirm').disabled),null,{timeout:30000,polling:100});
  if(await page.evaluate(()=>window.arpgChapterDownload?.status!=='ready' && document.querySelector('#downloads-dialog').open)){
   await page.waitForFunction(()=>!document.querySelector('#downloads-confirm').disabled);
   await capture('prepare');await page.locator('#downloads-confirm').click();
  }
  await page.waitForFunction(()=>window.arpgViewState==='game',null,{timeout:240000,polling:100});
  await page.waitForTimeout(5000);await capture('game');
  if(mobile && process.env.TOUCH==='1'){
   const before=await page.locator('canvas').boundingBox();const cdp=await ctx.newCDPSession(page);
   await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:120,y:180,id:1}]});
   await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:165,y:160,id:1}]});
   await page.waitForTimeout(1000);await capture('movement-held');
   await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
   await page.waitForTimeout(800);await capture('movement-cancelled');
   assert.deepEqual(await page.locator('canvas').boundingBox(),before,'Pad gestures must not shift the canvas');
  }
  if(process.env.GAME_SETTINGS==='1'){
   await page.touchscreen.tap(42,22);await page.waitForTimeout(1200);await capture('game-settings');
   await page.touchscreen.tap(page.viewportSize().width/2,Number(process.env.DOWNLOADS_Y||240));
   await page.waitForFunction(()=>document.querySelector('#downloads-dialog').open,null,{timeout:10000,polling:100});
   await capture('game-downloads');
   await page.locator('#downloads-close').click();
   await page.waitForTimeout(300);await page.touchscreen.tap(page.viewportSize().width/2,295);
   await page.waitForTimeout(1000);await capture('game-resumed');
  }
  assert.deepEqual(errors,[]);
  const receipt = await page.evaluate(async()=>{const registration=await navigator.serviceWorker.ready;return new Promise((resolve,reject)=>{const c=new MessageChannel(),t=setTimeout(()=>reject(new Error('Worker receipt timed out')),15000);c.port1.onmessage=({data})=>{clearTimeout(t);c.port1.close();resolve(data);};registration.active.postMessage({type:'STATUS',target:'current',wasmEncoding:'gzip'},[c.port2]);});});
  if(process.env.EXPECTED_REV)assert.equal(receipt.revision,process.env.EXPECTED_REV);
  await fs.writeFile(path.join(output,'result.json'),JSON.stringify({errors,requests,offline,mobile,gameSettings:process.env.GAME_SETTINGS==='1',receipt,state:await page.evaluate(()=>window.arpgViewState)},null,2));
  console.log('PASS real game entry',offline?'cold offline':'online',mobile?'mobile':'desktop');
 }catch(error){
  await page.screenshot({path:path.join(output,'failure.png'),timeout:30000}).catch(()=>{});
  await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({error:String(error),errors,requests,state:await page.evaluate(()=>({view:window.arpgViewState,download:window.arpgChapterDownload})).catch(()=>null)},null,2));
  throw error;
 }finally{await ctx.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
