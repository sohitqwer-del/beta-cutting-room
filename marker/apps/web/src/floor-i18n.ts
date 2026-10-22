import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';
import type { FloorLanguage } from './floor-types';
import { findAll } from './screens';
export type LabelKey = keyof typeof en;
export const languages = {en,hi,ta};
export function label(language:FloorLanguage,key:LabelKey):string {return languages[language][key] || en[key];}
/** Main navigation and new Floor labels only; existing editor controls retain their terms. */
export function translateLabels(language:FloorLanguage):void {
  findAll<HTMLElement>('[data-floor-label]').forEach(e=>{const key=e.dataset.floorLabel as LabelKey;if(key in en)e.textContent=label(language,key);e.lang=language;});
  findAll<HTMLElement>('[data-floor-title]').forEach(e=>{const key=e.dataset.floorTitle as LabelKey;if(key in en)e.title=label(language,key);});
  const tabs:Record<string,LabelKey>={'0':'marker_files','1':'cut_plan','2':'compare','3':'help','4':'pattern_formulas','5':'digitize','6':'rolls','7':'bundles','8':'cutting_floor','9':'home','10':'reports'};
  // Sidebar buttons keep their icon: only the label span changes.
  findAll<HTMLButtonElement>('[data-screen]').forEach(e=>{const key=tabs[e.dataset.screen??''];if(key){(e.querySelector('.nav-label')??e).textContent=label(language,key);e.lang=language;}});
  document.documentElement.dataset.language=language;
}
