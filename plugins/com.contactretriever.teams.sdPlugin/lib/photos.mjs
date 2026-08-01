/** Profile photo download via Teams profilepicturev2 and Microsoft Graph. */

import {
	GRAPH,
	TEAMS_PROFILE_BASE,
	SKYPE_AUD,
	cleanToken,
	tokenClaim,
	tokenExpirationDetails,
	describeTokens,
} from "./tokens.mjs";
import { emailOf } from "./search.mjs";

function toDataUrl(buffer, contentType) {
	const ctype = (contentType || "image/jpeg").split(";")[0].trim() || "image/jpeg";
	return `data:${ctype};base64,${Buffer.from(buffer).toString("base64")}`;
}

async function downloadPhotoTeams(graphOrAnyToken, skypeToken, person, name, teamsPart) {
	const photoToken = cleanToken(skypeToken);
	const aud = String(tokenClaim(photoToken, "aud", "") || "");
	if (!photoToken || !aud.includes(SKYPE_AUD)) return null;

	const actorOid = tokenClaim(photoToken, "oid") || tokenClaim(graphOrAnyToken, "oid");
	if (!actorOid) return null;

	const targetId = person.id || "";
	const targetMri = person.mri || (targetId ? `8:orgid:${targetId}` : "");
	if (!targetMri) return null;

	const size = "HR432x432";
	const display = encodeURIComponent(name || person.displayName || "contact");
	const part = teamsPart || "emea-02";
	const url =
		`${TEAMS_PROFILE_BASE}/${part}/beta/users/${actorOid}/profilepicturev2/${targetMri}` +
		`?displayname=${display}&size=${size}`;

	const authtokenValue = encodeURIComponent(
		`Bearer=${photoToken}&origin=https://teams.microsoft.com`,
	);

	const res = await fetch(url, {
		headers: {
			Accept: "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
			Referer: "https://teams.microsoft.com/v2/",
			"User-Agent":
				"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
			Cookie: `authtoken=${authtokenValue}; clienttype=web`,
		},
	});

	if (res.status === 401 || res.status === 403) {
		const err = new Error(
			`Photo Teams refusee (HTTP ${res.status}). ${tokenExpirationDetails(photoToken)}`,
		);
		err.code = "PHOTO_TEAMS_AUTH";
		throw err;
	}
	if (res.status === 404) return null;
	if (!res.ok) {
		const err = new Error(`Photo Teams echec HTTP ${res.status}`);
		err.code = "PHOTO_TEAMS_HTTP";
		throw err;
	}

	const ctype = res.headers.get("content-type") || "";
	const buf = Buffer.from(await res.arrayBuffer());
	if (!ctype.toLowerCase().startsWith("image/") || !buf.length) return null;
	return { dataUrl: toDataUrl(buf, ctype), source: "teams", bytes: buf.length };
}

async function downloadPhotoGraph(token, person) {
	const tok = cleanToken(token);
	if (!tok) return null;
	const ident = person.id || emailOf(person) || person.mail;
	if (!ident) return null;

	const res = await fetch(`${GRAPH}/users/${encodeURIComponent(ident)}/photo/$value`, {
		headers: { Authorization: `Bearer ${tok}` },
	});
	if (res.status === 401) {
		const err = new Error(`Photo Graph refusee (401). ${tokenExpirationDetails(tok)}`);
		err.code = "PHOTO_GRAPH_AUTH";
		throw err;
	}
	if (res.status === 404) return null;
	if (!res.ok) {
		const err = new Error(`Photo Graph echec HTTP ${res.status}`);
		err.code = "PHOTO_GRAPH_HTTP";
		throw err;
	}
	const ctype = res.headers.get("content-type") || "image/jpeg";
	const buf = Buffer.from(await res.arrayBuffer());
	if (!buf.length) return null;
	return { dataUrl: toDataUrl(buf, ctype), source: "graph", bytes: buf.length };
}

/**
 * Prefer Teams profilepicturev2 when a skype token is present (most consistent
 * with Teams web), otherwise fall back to Microsoft Graph.
 */
export async function downloadPhoto(globalSettings, person) {
	const info = describeTokens(globalSettings);
	const name = person.displayName || "Contact";

	if (info.hasSkype) {
		try {
			const teams = await downloadPhotoTeams(
				info.graphToken,
				info.skypeToken,
				person,
				name,
				info.teamsPart,
			);
			if (teams) return teams;
		} catch (e) {
			if (e.code === "PHOTO_TEAMS_AUTH") throw e;
			// fall through to Graph
		}
	}

	if (info.graphToken && info.backend === "graph") {
		return downloadPhotoGraph(info.graphToken, person);
	}

	// Substrate-only search token cannot fetch Graph photos.
	if (info.hasSkype) return null;
	if (info.graphToken) return downloadPhotoGraph(info.graphToken, person);
	return null;
}
