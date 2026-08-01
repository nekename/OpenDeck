/** Token helpers (JWT decode, audience detection, normalisation). */

export const GRAPH = "https://graph.microsoft.com/v1.0";
export const SUBSTRATE = "https://substrate.office.com/search/api/v1/suggestions";
export const TEAMS_PROFILE_BASE = "https://teams.microsoft.com/api/mt/part";
export const SKYPE_AUD = "https://api.spaces.skype.com";

export function cleanToken(tok) {
	if (!tok) return "";
	tok = String(tok).replace(/\s+/g, "");
	if (tok.slice(0, 6).toLowerCase() === "bearer") tok = tok.slice(6);
	return tok.replace(/^=+/, "").trim();
}

export function decodeJwtPayload(token) {
	const parts = String(token).split(".");
	if (parts.length < 2) throw new Error("format JWT non reconnu");
	const payloadB64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
	const padding = "=".repeat((4 - (payloadB64.length % 4)) % 4);
	const json = Buffer.from(payloadB64 + padding, "base64").toString("utf8");
	return JSON.parse(json);
}

export function tokenClaim(token, name, fallback = null) {
	try {
		const payload = decodeJwtPayload(token);
		return payload[name] ?? fallback;
	} catch {
		return fallback;
	}
}

export function tokenAudience(token) {
	try {
		const aud = tokenClaim(token, "aud");
		if (!aud) return "audience inconnue (champ 'aud' absent)";
		return String(aud);
	} catch (e) {
		return `audience inconnue (${e.message})`;
	}
}

export function tokenExpirationDetails(token) {
	try {
		const exp = tokenClaim(token, "exp");
		if (typeof exp !== "number") return "expiration inconnue (champ 'exp' absent)";
		const expMs = exp * 1000;
		const delta = Math.floor((expMs - Date.now()) / 1000);
		const stamp = new Date(expMs).toISOString().replace("T", " ").replace(/\.\d+Z$/, " UTC");
		if (delta >= 0) return `expire le ${stamp} (dans ~${delta} s)`;
		return `a expire le ${stamp} (il y a ~${Math.abs(delta)} s)`;
	} catch (e) {
		return `expiration inconnue (${e.message})`;
	}
}

/** graph | substrate | unknown — pick search backend from token audience. */
export function detectBackend(token) {
	const aud = String(tokenClaim(token, "aud", "") || "");
	if (aud.includes("graph.microsoft.com")) return "graph";
	if (aud.includes("outlook.office.com/search")) return "substrate";
	return "unknown";
}

export function describeTokens(globalSettings = {}) {
	const graphToken = cleanToken(globalSettings.graphToken);
	const skypeToken = cleanToken(globalSettings.skypeToken);
	const backend = graphToken ? detectBackend(graphToken) : "none";
	return {
		graphToken,
		skypeToken,
		backend,
		graphAud: graphToken ? tokenAudience(graphToken) : "",
		graphExp: graphToken ? tokenExpirationDetails(graphToken) : "",
		skypeAud: skypeToken ? tokenAudience(skypeToken) : "",
		skypeExp: skypeToken ? tokenExpirationDetails(skypeToken) : "",
		hasSkype: Boolean(skypeToken && String(tokenClaim(skypeToken, "aud", "")).includes(SKYPE_AUD)),
		teamsPart: globalSettings.teamsPart || "emea-02",
		doubleClickMs: Number(globalSettings.doubleClickMs) > 0 ? Number(globalSettings.doubleClickMs) : 400,
	};
}
