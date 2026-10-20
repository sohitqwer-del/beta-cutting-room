import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';
import type { FloorLanguage } from './floor-types';
export type LabelKey = keyof typeof en;
export const languages = {en,hi,ta};
export function label(language:FloorLanguage,key:LabelKey):string {return languages[language][key] || en[key];}
/** Main navigation and new Floor labels only; existing editor controls retain their terms. */
export function translateLabels(language:FloorLanguage):void {
  document.querySelectorAll<HTMLElement>('[data-floor-label]').forEach(e=>{const key=e.dataset.floorLabel as LabelKey;if(key in en)e.textContent=label(language,key);e.lang=language;});
  const tabs:Record<string,LabelKey>={'0':'editor','1':'cut_plan','2':'compare','3':'help','4':'pattern','5':'digitize','6':'rolls','7':'bundles','8':'floor','9':'home','10':'reports'};
  // Sidebar buttons keep their icon: only the label span changes.
  document.querySelectorAll<HTMLButtonElement>('[data-screen]').forEach(e=>{const key=tabs[e.dataset.screen??''];if(key){(e.querySelector('.nav-label')??e).textContent=label(language,key);e.lang=language;}});
  document.documentElement.dataset.language=language;
}
