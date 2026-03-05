import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

type LocaleModule = { default: Record<string, unknown> };

const languageAliases: Record<string, string> = {
  'zh-CN': 'zh-cn',
  'en-US': 'en',
};

export const supportedLanguages = [
  'zh-cn',
  'en',
];

const localeLoaders: Record<string, () => Promise<LocaleModule>> = {
  'zh-cn': () => import('../locales/zh-CN.json'),
  en: () => import('../locales/en.json'),
};

const loadedLanguages = new Set<string>();
let initPromise: Promise<void> | null = null;

export function normalizeLanguage(lang: string): string {
  const trimmed = lang.trim();
  if (!trimmed) {
    return 'zh-cn';
  }

  if (languageAliases[trimmed]) {
    return languageAliases[trimmed];
  }

  const lower = trimmed.toLowerCase();
  if (languageAliases[lower]) {
    return languageAliases[lower];
  }

  return lower;
}

function resolveSupportedLanguage(lang: string): string {
  const normalized = normalizeLanguage(lang);
  return supportedLanguages.includes(normalized) ? normalized : 'zh-cn';
}

async function ensureLanguageResources(lang: string): Promise<string> {
  const resolved = resolveSupportedLanguage(lang);
  if (loadedLanguages.has(resolved)) {
    return resolved;
  }

  const loader = localeLoaders[resolved] ?? localeLoaders['zh-cn'];
  const module = await loader();
  i18n.addResourceBundle(resolved, 'translation', module.default, true, true);
  loadedLanguages.add(resolved);
  return resolved;
}

export async function initI18n(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    const savedLanguage = resolveSupportedLanguage(
      localStorage.getItem('app-language') || 'zh-cn',
    );

    await i18n
      .use(initReactI18next)
      .init({
        resources: {},
        lng: 'zh-cn',
        fallbackLng: 'zh-cn',
        supportedLngs: supportedLanguages,
        lowerCaseLng: true,
        load: 'currentOnly',
        interpolation: {
          escapeValue: false, // React 已经处理了 XSS
        },
      });

    await ensureLanguageResources('zh-cn');
    if (savedLanguage !== 'zh-cn') {
      await ensureLanguageResources(savedLanguage);
    }
    await i18n.changeLanguage(savedLanguage);
  })();

  return initPromise;
}

void initI18n();

/**
 * 切换语言
 */
export async function changeLanguage(lang: string): Promise<void> {
  const resolved = await ensureLanguageResources(lang);
  await i18n.changeLanguage(resolved);
  localStorage.setItem('app-language', resolved);
}

/**
 * 获取当前语言
 */
export function getCurrentLanguage(): string {
  return normalizeLanguage(i18n.language || 'zh-CN');
}

export default i18n;
