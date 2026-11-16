import {setInstallLanguage} from './pwa.js';
import {setCacheLanguage, prepareGameCache, markGameStarted, consumeCacheResume} from './game-cache.js';
const copy = {
  en: {intro:'Descend into the Ashen Reliquary. Shape your build. Defy the Hollow.', start:'Begin journey', loading:'Preparing…', ready:'Choose your descent.', resources:'Preparing saved resources…', engine:'Starting the engine…', world:'Preparing the menu…', error:'Unable to start. Check your connection and try again.', retry:'Retry', gpu:'WebGPU is unavailable. Try a browser with WebGPU support.', download:(a,b)=>`Engine · ${a}${b ? ' / '+b : ''} MB`},
  'zh-CN': {intro:'踏入灰烬圣所，构筑流派，挑战空洞之主。', start:'开始旅程', loading:'准备中…', ready:'选择你的远征。', resources:'正在准备已保存的资源…', engine:'正在启动引擎…', world:'正在准备主菜单…', error:'启动失败，请检查网络后重试。', retry:'重试', gpu:'当前浏览器无法使用 WebGPU，请换用支持 WebGPU 的浏览器。', download:(a,b)=>`引擎 · ${a}${b ? ' / '+b : ''} MB`},
};
let language='en', phase='ready', busy=false, engineStarted=false, received=0, total=0;
try { language=new URLSearchParams(location.search).get('lang') || localStorage.getItem('open-arpg-language') || 'en'; } catch {}
if (!copy[language]) language='en';
function render() {
  const t=copy[language];
  document.documentElement.lang=language;
  document.querySelector('#intro').textContent=t.intro;
  document.querySelector('#status').textContent=phase==='download' ? t.download((received/1048576).toFixed(1),total ? (total/1048576).toFixed(1):'') : t[phase];
  document.querySelector('#start').textContent=busy?t.loading:phase==='error'?t.retry:t.start;
  for (const button of document.querySelectorAll('[data-language]')) button.setAttribute('aria-pressed',String(button.dataset.language===language));
  setInstallLanguage(language); setCacheLanguage(language);
}
window.arpgSetLanguage=value=>{
  language=copy[value]?value:'en';
  try {localStorage.setItem('open-arpg-language',language);} catch {}
  render();
};
for (const button of document.querySelectorAll('[data-language]')) button.addEventListener('click',()=>window.arpgSetLanguage(button.dataset.language));
async function downloadEngine(buildId) {
  if (typeof DecompressionStream!=='function') return undefined;
  const response=await fetch(`./pkg/bevy_open_arpg_bg.wasm.gz?v=${buildId}`);
  if (!response.ok || !response.body) throw Error('Engine download failed');
  total=Number(response.headers.get('content-length')||0);received=0;phase='download';render();
  const progress=new TransformStream({transform(chunk,controller){received+=chunk.byteLength;render();controller.enqueue(chunk);}});
  return new Response(response.body.pipeThrough(progress).pipeThrough(new DecompressionStream('gzip'))).arrayBuffer();
}
export function boot(buildId) {
  window.arpgAssetFailure=()=>{
    window.arpgLoadError=true;
    phase='error';busy=false;
    document.querySelector('#start').disabled=false;
    document.querySelector('#loading').hidden=false;
    document.body.classList.remove('playing');
    render();
  };
  render();
  const start=document.querySelector('#start');
  start.addEventListener('click',async()=>{
    if (busy) return;
    if (engineStarted) { location.reload(); return; }
    busy=true;start.disabled=true;phase='resources';render();
    try {
      const adapter=await navigator.gpu?.requestAdapter();
      if (!adapter) {phase='gpu';return;}
      await prepareGameCache();
      const {default:init}=await import(`./pkg/bevy_open_arpg.js?v=${buildId}`);
      const bytes=await downloadEngine(buildId);phase='engine';render();
      engineStarted=true;
      try {await init({module_or_path:bytes || `./pkg/bevy_open_arpg_bg.wasm?v=${buildId}`});}
      catch(error){if (!String(error).includes('Using exceptions for control flow'))throw error;}
      phase='world';render();
      // Runtime reports the real asset/state boundary. No elapsed-time bypass.
      await new Promise((resolve,reject)=>{
        const timer=setInterval(()=>{
          if (window.arpgLoadError) {clearInterval(timer);reject(Error('Menu assets failed to decode'));return;}
          if (window.arpgViewState==='menu' || window.arpgViewState==='game') {clearInterval(timer);resolve();}
        },100);
      });
      markGameStarted();document.body.classList.add('playing');document.querySelector('#loading').hidden=true;
      document.querySelector('#bevy-canvas').focus();
    } catch(error){console.error(error);phase='error';}
    finally {busy=false;start.disabled=false;render();}
  });
  if(consumeCacheResume())start.click();
}
