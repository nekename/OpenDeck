// Pages are stored as profiles with IDs like "Profile~2" ("Profile" being page 1).
// User-created profile names cannot contain the separator (enforced by the name pattern).
// These helpers must mirror `profile_page_parts`/`profile_page_id` in `src-tauri/src/store/profiles.rs`.

export const PAGE_SEPARATOR = "~";

export function pageParts(id: string): [string, number] {
	const index = id.lastIndexOf(PAGE_SEPARATOR);
	if (index != -1) {
		const number = parseInt(id.slice(index + 1));
		if (!isNaN(number) && number >= 2 && /^\d+$/.test(id.slice(index + 1))) {
			return [id.slice(0, index), number];
		}
	}
	return [id, 1];
}

export function pageId(base: string, page: number): string {
	return page <= 1 ? base : base + PAGE_SEPARATOR + page;
}

export function isPage(id: string): boolean {
	return pageParts(id)[1] > 1;
}
