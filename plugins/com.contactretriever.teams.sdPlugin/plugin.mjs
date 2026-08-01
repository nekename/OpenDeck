#!/usr/bin/env node
/**
 * Teams Contact — OpenDeck / OpenAction plugin
 *
 * Global settings: graphToken, skypeToken, teamsPart, doubleClickMs
 * Instance settings: displayName, mail, id, mri, photoDataUrl
 *
 * Single click  -> msteams chat
 * Double click  -> msteams audio call
 */

import { searchContacts, emailOf } from "./lib/search.mjs";
import { downloadPhoto } from "./lib/photos.mjs";
import { describeTokens, cleanToken } from "./lib/tokens.mjs";

const ACTION_UUID = "com.contactretriever.teams.contact";

function argValue(name) {
	const i = process.argv.indexOf(name);
	return i >= 0 ? process.argv[i + 1] : null;
}

const port = argValue("-port");
const pluginUUID = argValue("-pluginUUID");
const registerEvent = argValue("-registerEvent") || "registerPlugin";

if (!port || !pluginUUID) {
	console.error("Usage: node plugin.mjs -port <port> -pluginUUID <uuid> -registerEvent registerPlugin -info <json>");
	process.exit(1);
}

const WS = await (async () => {
	if (typeof WebSocket !== "undefined") return WebSocket;
	try {
		const mod = await import("ws");
		return mod.default || mod.WebSocket;
	} catch {
		console.error("WebSocket manquant: utilise Node.js 22+ ou lance `npm install` dans le dossier du plugin.");
		process.exit(1);
	}
})();

let globalSettings = {};
/** @type {Map<string, {timer: NodeJS.Timeout, settings: object}>} */
const pendingClicks = new Map();
/** Keep last instance settings for click handling if payload is thin. */
const instanceCache = new Map();

function send(msg) {
	if (ws.readyState === WS.OPEN) ws.send(JSON.stringify(msg));
}

function log(message) {
	console.log(`[teams-contact] ${message}`);
	send({ event: "logMessage", payload: { message: String(message) } });
}

function setTitle(context, title) {
	send({
		event: "setTitle",
		context,
		payload: { title: title || "", target: 0 },
	});
}

function setImage(context, image) {
	if (!image) return;
	send({
		event: "setImage",
		context,
		payload: { image, target: 0 },
	});
}

function setSettings(context, settings) {
	instanceCache.set(context, settings);
	send({ event: "setSettings", context, payload: settings });
}

function showAlert(context) {
	send({ event: "showAlert", context });
}

function showOk(context) {
	send({ event: "showOk", context });
}

function openUrl(url) {
	send({ event: "openUrl", payload: { url } });
}

function sendToPi(context, payload) {
	send({
		event: "sendToPropertyInspector",
		action: ACTION_UUID,
		context,
		payload,
	});
}

function shortTitle(name, email) {
	const base = (name || email || "").trim();
	if (!base) return "";
	if (base.length <= 14) return base;
	return base.slice(0, 12) + "…";
}

function chatUrl(email) {
	return `msteams://teams.microsoft.com/l/chat/0/0?users=${encodeURIComponent(email)}`;
}

function callUrl(email) {
	// Audio call (no withVideo)
	return `msteams://teams.microsoft.com/l/call/0/0?users=${encodeURIComponent(email)}`;
}

function applyContactVisual(context, settings) {
	if (settings?.photoDataUrl) setImage(context, settings.photoDataUrl);
	setTitle(context, shortTitle(settings?.displayName, settings?.mail));
}

async function refreshPhoto(context, settings) {
	const person = {
		id: settings.id || "",
		mri: settings.mri || "",
		displayName: settings.displayName || "",
		mail: settings.mail || "",
	};
	const photo = await downloadPhoto(globalSettings, person);
	if (!photo) return { ...settings, photoDataUrl: settings.photoDataUrl || "" };
	const next = { ...settings, photoDataUrl: photo.dataUrl, photoSource: photo.source };
	setSettings(context, next);
	setImage(context, photo.dataUrl);
	return next;
}

async function handleSelectContact(context, person) {
	const mail = person.mail || emailOf(person) || "";
	const settings = {
		displayName: person.displayName || "Contact",
		mail,
		id: person.id || "",
		mri: person.mri || (person.id ? `8:orgid:${person.id}` : ""),
		photoDataUrl: "",
		photoSource: "",
	};
	setSettings(context, settings);
	setTitle(context, shortTitle(settings.displayName, settings.mail));

	try {
		const withPhoto = await refreshPhoto(context, settings);
		sendToPi(context, {
			event: "contactSelected",
			ok: true,
			settings: {
				displayName: withPhoto.displayName,
				mail: withPhoto.mail,
				id: withPhoto.id,
				mri: withPhoto.mri,
				photoSource: withPhoto.photoSource || "",
				hasPhoto: Boolean(withPhoto.photoDataUrl),
			},
		});
		showOk(context);
	} catch (e) {
		log(`Photo: ${e.message}`);
		sendToPi(context, {
			event: "contactSelected",
			ok: true,
			warning: e.message,
			settings: {
				displayName: settings.displayName,
				mail: settings.mail,
				id: settings.id,
				mri: settings.mri,
				hasPhoto: false,
			},
		});
		showAlert(context);
	}
}

async function handleSearch(context, query) {
	const info = describeTokens(globalSettings);
	if (!info.graphToken) {
		sendToPi(context, {
			event: "searchResults",
			ok: false,
			error: "Token de recherche manquant. Colle le JWT Graph ou Substrate dans les reglages globaux.",
		});
		return;
	}
	try {
		const { people, source, backend } = await searchContacts(info.graphToken, query);
		sendToPi(context, {
			event: "searchResults",
			ok: true,
			source,
			backend,
			people: people.map((p) => ({
				id: p.id,
				mri: p.mri,
				displayName: p.displayName,
				mail: p.mail,
				userPrincipalName: p.userPrincipalName,
			})),
		});
	} catch (e) {
		log(`Search error: ${e.message}`);
		sendToPi(context, { event: "searchResults", ok: false, error: e.message });
	}
}

function handleKeyUp(context, settings) {
	const email = (settings?.mail || "").trim();
	if (!email) {
		showAlert(context);
		log("Aucun contact configure sur cette touche.");
		return;
	}

	const info = describeTokens(globalSettings);
	const windowMs = info.doubleClickMs;

	const existing = pendingClicks.get(context);
	if (existing) {
		clearTimeout(existing.timer);
		pendingClicks.delete(context);
		const url = callUrl(email);
		log(`Double-click call -> ${url}`);
		openUrl(url);
		return;
	}

	const timer = setTimeout(() => {
		pendingClicks.delete(context);
		const url = chatUrl(email);
		log(`Single-click chat -> ${url}`);
		openUrl(url);
	}, windowMs);
	pendingClicks.set(context, { timer, settings });
}

function normalizeContext(ctx) {
	if (ctx == null) return null;
	if (typeof ctx === "string") return ctx;
	if (typeof ctx === "object" && ctx.device != null) {
		return `${ctx.device}.${ctx.profile}.${ctx.controller}.${ctx.position}.${ctx.index ?? 0}`;
	}
	return String(ctx);
}

function pushTokenStatus(context) {
	const info = describeTokens(globalSettings);
	sendToPi(context, {
		event: "tokenStatus",
		backend: info.backend,
		graphAud: info.graphAud,
		graphExp: info.graphExp,
		skypeAud: info.skypeAud,
		skypeExp: info.skypeExp,
		hasSkype: info.hasSkype,
		teamsPart: info.teamsPart,
		doubleClickMs: info.doubleClickMs,
		hasGraphToken: Boolean(info.graphToken),
	});
}

const ws = new WS(`ws://127.0.0.1:${port}`);

function onOpen() {
	send({ event: registerEvent, uuid: pluginUUID });
	log(`Registered ${pluginUUID}`);
	send({ event: "getGlobalSettings", context: pluginUUID });
}

async function onMessage(raw) {
	let msg;
	try {
		msg = JSON.parse(typeof raw === "string" ? raw : raw.toString());
	} catch {
		return;
	}

	const { event } = msg;
	const context = normalizeContext(msg.context);

	switch (event) {
		case "didReceiveGlobalSettings": {
			globalSettings = msg.payload?.settings || {};
			break;
		}
		case "didReceiveSettings": {
			if (context) instanceCache.set(context, msg.payload?.settings || {});
			break;
		}
		case "willAppear": {
			const settings = msg.payload?.settings || {};
			if (context) {
				instanceCache.set(context, settings);
				applyContactVisual(context, settings);
			}
			break;
		}
		case "willDisappear": {
			if (context) {
				const pending = pendingClicks.get(context);
				if (pending) {
					clearTimeout(pending.timer);
					pendingClicks.delete(context);
				}
			}
			break;
		}
		case "keyUp": {
			const settings = msg.payload?.settings || instanceCache.get(context) || {};
			handleKeyUp(context, settings);
			break;
		}
		case "keyDown": {
			// Intentionally empty: action runs on keyUp for click timing.
			break;
		}
		case "propertyInspectorDidAppear": {
			if (context) {
				send({ event: "getGlobalSettings", context: pluginUUID });
				pushTokenStatus(context);
			}
			break;
		}
		case "sendToPlugin": {
			const payload = msg.payload || {};
			const type = payload.event || payload.type;
			if (!context) break;

			if (type === "saveGlobal") {
				globalSettings = {
					graphToken: cleanToken(payload.graphToken),
					skypeToken: cleanToken(payload.skypeToken),
					teamsPart: (payload.teamsPart || "emea-02").trim() || "emea-02",
					doubleClickMs: Number(payload.doubleClickMs) > 0 ? Number(payload.doubleClickMs) : 400,
				};
				send({ event: "setGlobalSettings", context: pluginUUID, payload: globalSettings });
				pushTokenStatus(context);
				sendToPi(context, { event: "globalSaved", ok: true });
				break;
			}

			if (type === "getTokenStatus") {
				pushTokenStatus(context);
				break;
			}

			if (type === "search") {
				await handleSearch(context, payload.query || "");
				break;
			}

			if (type === "selectContact") {
				await handleSelectContact(context, payload.person || {});
				break;
			}

			if (type === "refreshPhoto") {
				const settings = instanceCache.get(context) || payload.settings || {};
				try {
					const next = await refreshPhoto(context, settings);
					sendToPi(context, {
						event: "photoRefreshed",
						ok: Boolean(next.photoDataUrl),
						photoSource: next.photoSource || "",
					});
					if (next.photoDataUrl) showOk(context);
					else showAlert(context);
				} catch (e) {
					sendToPi(context, { event: "photoRefreshed", ok: false, error: e.message });
					showAlert(context);
				}
				break;
			}
			break;
		}
		default:
			break;
	}
}

if (typeof ws.addEventListener === "function") {
	ws.addEventListener("open", onOpen);
	ws.addEventListener("message", (ev) => onMessage(ev.data));
	ws.addEventListener("close", () => {
		log("WebSocket closed");
		process.exit(0);
	});
	ws.addEventListener("error", (err) => {
		console.error("WebSocket error", err.message || err);
	});
} else {
	// `ws` package API
	ws.on("open", onOpen);
	ws.on("message", (data) => onMessage(data));
	ws.on("close", () => {
		log("WebSocket closed");
		process.exit(0);
	});
	ws.on("error", (err) => {
		console.error("WebSocket error", err.message || err);
	});
}
