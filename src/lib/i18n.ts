import { derived, writable } from "svelte/store";

const FALLBACK_LOCALE = "en";

const translationModules = import.meta.glob("../../translations/*.json", {
	eager: true,
	import: "default",
}) as Record<string, Record<string, string>>;

const translations: Record<string, Record<string, string>> = Object.fromEntries(
	Object.entries(translationModules).map(([path, translations]) => {
		const locale = path.match(/\/([^/]+)\.json$/)?.[1];

		if (!locale) {
			throw new Error(`Could not determine locale from translation file path: ${path}`);
		}

		return [locale, translations];
	}),
);

export const locale = writable("en");

function translate(locale: string, key: string, vars: { [x: string]: any }) {
	if (!key) throw new Error("no key provided to $t()");
	if (!locale) throw new Error("no locale provided to $t()");

	// Grab the translation from the translation object.
	const dictionary = translations as Record<string, Record<string, string>>;
	let text = dictionary[locale]?.[key] ?? dictionary[FALLBACK_LOCALE]?.[key];

	if (!text) throw new Error(`no translation found for ${locale}.${key} or ${FALLBACK_LOCALE}.${key}`);

	// Replace any passed in variables in the translation string.
	Object.keys(vars).map((k) => {
		const regex = new RegExp(`{{${k}}}`, "g");
		text = text.replace(regex, vars[k]);
	});

	return text;
}

export const t = derived(locale, ($locale) => (key: string, vars = {}) => translate($locale, key, vars));
