/* Small dependency-free i18n runtime. Locale dictionaries live in ../locales/*.json. */
(() => {
  const supportedLanguages = [];
  let languageNames = {};
  const dictionaries = {};
  const reverseKeys = {};
  const textNodeKeys = new WeakMap();
  let currentLanguage = "en";

  async function load(language) {
    if (!supportedLanguages.includes(language)) language = "en";
    if (!dictionaries[language]) {
      const response = await fetch(`./locales/${language}.json`);
      if (!response.ok) throw new Error(`Could not load locale: ${language}`);
      dictionaries[language] = await response.json();
      reverseKeys[language] = new Map(Object.entries(dictionaries[language]).map(([key, value]) => [value, key]));
    }
    return dictionaries[language];
  }

  function t(key, language = currentLanguage) {
    return dictionaries[language]?.[key] ?? dictionaries.en?.[key] ?? key;
  }

  function keyForText(value) {
    if (Object.hasOwn(dictionaries.en || {}, value)) return value;
    for (const language of supportedLanguages) {
      const key = reverseKeys[language]?.get(value);
      if (key) return key;
    }
    return null;
  }

  function translateValue(value) {
    const source = String(value);
    const exactKey = keyForText(source);
    if (exactKey) return t(exactKey);
    if (source.startsWith("Error: ")) return `Error: ${translateValue(source.slice(7))}`;
    for (const language of supportedLanguages) {
      for (const [translated, key] of reverseKeys[language] || []) {
        if (translated.endsWith(": ") && source.startsWith(translated)) {
          return `${t(key)}${source.slice(translated.length)}`;
        }
      }
    }
    return source;
  }

  function populateLanguageSelector() {
    const selector = document.getElementById("language-select");
    if (!selector) return;
    const selected = selector.value;
    selector.replaceChildren();
    for (const [code, label] of Object.entries(languageNames)) {
      const option = document.createElement("option");
      option.value = code;
      option.textContent = label;
      selector.append(option);
    }
    if (supportedLanguages.includes(selected)) selector.value = selected;
  }

  async function initialize() {
    const response = await fetch("./locales/manifest.json");
    if (!response.ok) throw new Error("Could not load locale manifest");
    languageNames = await response.json();
    supportedLanguages.push(...Object.keys(languageNames));
    await Promise.all(supportedLanguages.map(load));
    populateLanguageSelector();
  }

  const ready = initialize();

  async function setLanguage(language) {
    await ready;
    currentLanguage = supportedLanguages.includes(language) ? language : "en";
    translatePage();
    document.documentElement.lang = currentLanguage;
    return currentLanguage;
  }

  function translatePage() {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    const textNodes = [];
    let node;
    while ((node = walker.nextNode())) textNodes.push(node);
    for (const textNode of textNodes) {
      const raw = textNode.nodeValue;
      const trimmed = raw.trim();
      if (!trimmed) continue;
      const key = textNodeKeys.get(textNode) || keyForText(trimmed);
      if (!key) continue;
      textNodeKeys.set(textNode, key);
      textNode.nodeValue = raw.replace(trimmed, t(key));
    }
    document.querySelectorAll("[aria-label], [title]").forEach((element) => {
      for (const attribute of ["aria-label", "title"]) {
        const value = element.getAttribute(attribute);
        if (!value) continue;
        const dataKey = `i18n${attribute.replace(/(^|-)(.)/g, (_, __, c) => c.toUpperCase())}`;
        if (!element.dataset[dataKey]) {
          const key = keyForText(value);
          if (key) element.dataset[dataKey] = key;
        }
        if (element.dataset[dataKey]) element.setAttribute(attribute, t(element.dataset[dataKey]));
      }
    });
    document.querySelectorAll("input[placeholder]").forEach((input) => {
      if (!input.dataset.i18nPlaceholder) {
        const placeholder = input.placeholder;
        const key = keyForText(placeholder);
        if (key) input.dataset.i18nPlaceholder = key;
      }
      const key = input.dataset.i18nPlaceholder;
      if (key) input.placeholder = t(key);
    });
  }

  window.NexusQI18n = { supportedLanguages, languageNames: () => ({ ...languageNames }), load, ready, t, translateValue, setLanguage, translatePage, get language() { return currentLanguage; } };
})();
