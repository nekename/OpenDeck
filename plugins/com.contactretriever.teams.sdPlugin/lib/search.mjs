/** Contact search via Microsoft Graph and Teams Substrate (Powerbar). */

import { randomUUID } from "node:crypto";
import {
	GRAPH,
	SUBSTRATE,
	cleanToken,
	detectBackend,
	tokenClaim,
	tokenAudience,
	tokenExpirationDetails,
} from "./tokens.mjs";

function authError(token, label) {
	const err = new Error(
		`401 : token ${label} invalide ou expire.\n` +
			`Etat : ${tokenExpirationDetails(token)}\n` +
			`Audience : ${tokenAudience(token)}`,
	);
	err.code = "TOKEN_401";
	return err;
}

async function graphGet(token, url, params = {}, extraHeaders = {}) {
	const qs = new URLSearchParams(params);
	const full = qs.toString() ? `${url}?${qs}` : url;
	const res = await fetch(full, {
		headers: {
			Authorization: `Bearer ${token}`,
			...extraHeaders,
		},
	});
	if (res.status === 401) throw authError(token, "Graph");
	return res;
}

async function substratePost(token, body, scenario = "powerbar") {
	const url =
		`${SUBSTRATE}?scenario=${scenario}` +
		"&setflight=EnableMessageImageSearch,EnableTeamsChannelDomainPowerbar";

	const headers = {
		Authorization: `Bearer ${token}`,
		"Content-Type": "application/json",
		"x-client-version": "T2.1",
		Referer: "https://teams.microsoft.com/",
		"User-Agent":
			"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
	};

	const oid = tokenClaim(token, "oid");
	const tid = tokenClaim(token, "tid");
	if (oid && tid) headers["x-anchormailbox"] = `Oid:${oid}@${tid}`;

	const res = await fetch(url, {
		method: "POST",
		headers,
		body: JSON.stringify(body),
	});
	if (res.status === 401) throw authError(token, "Substrate");
	return res;
}

export function emailOf(person) {
	const addrs = person.scoredEmailAddresses || [];
	if (addrs[0]?.address) return addrs[0].address;
	return person.mail || person.userPrincipalName || "";
}

async function searchPeopleGraph(token, query) {
	const res = await graphGet(token, `${GRAPH}/me/people`, {
		$search: `"${query}"`,
		$top: "25",
	});
	if (!res.ok) return [];
	const data = await res.json();
	return (data.value || []).map((p) => ({
		id: p.id || "",
		mri: p.mri || (p.id ? `8:orgid:${p.id}` : ""),
		displayName: p.displayName || "?",
		mail: emailOf(p),
		userPrincipalName: p.userPrincipalName || "",
		_source: "graph-people",
	}));
}

async function searchDirectoryGraph(token, query) {
	const res = await graphGet(
		token,
		`${GRAPH}/users`,
		{
			$search: `"displayName:${query}"`,
			$top: "25",
			$select: "id,displayName,mail,userPrincipalName",
		},
		{ ConsistencyLevel: "eventual" },
	);
	if (!res.ok) return [];
	const data = await res.json();
	return (data.value || []).map((p) => ({
		id: p.id || "",
		mri: p.id ? `8:orgid:${p.id}` : "",
		displayName: p.displayName || "?",
		mail: p.mail || p.userPrincipalName || "",
		userPrincipalName: p.userPrincipalName || "",
		_source: "graph-directory",
	}));
}

function extractPeopleRows(obj, rows = []) {
	if (Array.isArray(obj)) {
		for (const item of obj) extractPeopleRows(item, rows);
		return rows;
	}
	if (obj && typeof obj === "object") {
		if (
			typeof obj.DisplayName === "string" &&
			(obj.EmailAddresses || obj.UserPrincipalName || obj.ExternalDirectoryObjectId)
		) {
			rows.push(obj);
		}
		for (const value of Object.values(obj)) extractPeopleRows(value, rows);
	}
	return rows;
}

function substrateEmailOf(row) {
	const addrs = row.EmailAddresses || [];
	if (Array.isArray(addrs) && addrs.length) {
		const first = addrs[0];
		if (first && typeof first === "object") {
			return first.Address || first.EmailAddress || "";
		}
		if (typeof first === "string") return first;
	}
	return row.UserPrincipalName || "";
}

function toPersonFromSubstrate(row) {
	const id = row.ExternalDirectoryObjectId || row.Id || "";
	return {
		id,
		mri: row.MRI || (id ? `8:orgid:${id}` : ""),
		displayName: row.DisplayName || "?",
		mail: substrateEmailOf(row),
		userPrincipalName: row.UserPrincipalName || "",
		_source: "substrate",
	};
}

async function searchPeopleSubstrate(token, query) {
	const peopleQuery = {
		QueryString: query,
		DisplayQueryString: query,
		NormalizedQueryString: query,
	};
	const simpleQuery = {
		QueryString: query,
		NormalizedQueryString: query,
	};

	const body = {
		EntityRequests: [
			{
				Query: peopleQuery,
				EntityType: "People",
				Size: 10,
				Fields: [
					"Id",
					"MRI",
					"DisplayName",
					"EmailAddresses",
					"PeopleType",
					"PeopleSubtype",
					"Phones",
					"GivenName",
					"Surname",
					"Cid",
					"CompanyName",
					"ImAddress",
					"UserPrincipalName",
					"ExternalDirectoryObjectId",
					"ConcatenatedId",
					"Department",
					"JobTitle",
				],
				Filter: {
					And: [
						{
							Or: [
								{ Term: { PeopleType: "Person" } },
								{ Term: { PeopleType: "Other" } },
							],
						},
						{
							Or: [
								{ Term: { PeopleSubtype: "OrganizationUser" } },
								{ Term: { PeopleSubtype: "MTOUser" } },
								{ Term: { PeopleSubtype: "PersonalContact" } },
								{ Term: { PeopleSubtype: "Guest" } },
							],
						},
						{ Or: [{ Term: { Flags: "NonHidden" } }] },
					],
				},
				Provenances: ["Mailbox", "Directory"],
				From: 0,
			},
			{ Query: simpleQuery, EntityType: "File", Size: 1 },
			{ Query: simpleQuery, EntityType: "Chat", Size: 1 },
		],
		Scenario: {
			Name: "powerbar",
			Dimensions: [{ DimensionName: "QueryType", DimensionValue: "PeopleCentricSearch" }],
		},
		Cvid: randomUUID(),
		AppName: "Microsoft Teams",
		LogicalId: randomUUID(),
		dataSource: "personScoped",
	};

	const res = await substratePost(token, body);
	if (!res.ok) {
		const snippet = (await res.text()).trim().replace(/\n/g, " ").slice(0, 400);
		const err = new Error(`Echec Substrate: HTTP ${res.status}${snippet ? ` — ${snippet}` : ""}`);
		err.code = "SUBSTRATE_HTTP";
		throw err;
	}

	const payload = await res.json();
	const uniq = new Map();
	for (const row of extractPeopleRows(payload)) {
		const person = toPersonFromSubstrate(row);
		const key = `${person.displayName}|${person.mail}|${person.id}`;
		if (!uniq.has(key)) uniq.set(key, person);
	}
	return [...uniq.values()];
}

/**
 * Search contacts using the most compatible backend for the provided token.
 * - Graph token  -> /me/people then /users directory
 * - Substrate token (Teams Powerbar) -> substrate suggestions
 */
export async function searchContacts(rawToken, query) {
	const token = cleanToken(rawToken);
	if (!token) throw new Error("Token de recherche manquant (colle-le dans les reglage globaux).");
	if (!query?.trim()) throw new Error("Requete vide.");

	const backend = detectBackend(token);
	if (backend === "substrate") {
		const people = await searchPeopleSubstrate(token, query.trim());
		return { people, source: "teams/substrate", backend };
	}

	if (backend === "unknown") {
		// Tentative Graph d'abord (le plus documente), puis Substrate si 401/echec total.
		try {
			let people = await searchPeopleGraph(token, query.trim());
			let source = "contacts";
			if (!people.length) {
				people = await searchDirectoryGraph(token, query.trim());
				source = "annuaire";
			}
			if (people.length) return { people, source, backend: "graph" };
		} catch (e) {
			if (e.code !== "TOKEN_401") throw e;
		}
		const people = await searchPeopleSubstrate(token, query.trim());
		return { people, source: "teams/substrate", backend: "substrate" };
	}

	let people = await searchPeopleGraph(token, query.trim());
	let source = "contacts";
	if (!people.length) {
		people = await searchDirectoryGraph(token, query.trim());
		source = "annuaire";
	}
	return { people, source, backend: "graph" };
}
